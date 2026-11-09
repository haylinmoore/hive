{ config, pkgs, ... }:
{
  # Services
  services.openssh.enable = false;
  services.upower.enable = true;
  services.fwupd.enable = true;
  services.printing.enable = true;
  services.lldpd.enable = true;
  services.tailscale.enable = true;
  services.fprintd.enable = true;
  services.logind.settings.Login.HandleLidSwitch = "suspend-then-hibernate";
  systemd.sleep.settings.Sleep.HibernateDelaySec = "1h";
  # S4 leaves xhci/thunderbolt armed and usb-c events power it back on
  systemd.sleep.settings.Sleep.HibernateMode = "shutdown";
  boot.initrd.systemd.targets.cryptsetup.unitConfig = {
    JobTimeoutSec = "5min";
    JobTimeoutAction = "poweroff";
  };

  # resuming the image needed the luks passphrase, so skip swaylock. the
  # restore message is pm_debug only and never printed if hibernate failed.
  boot.kernelParams = [ "pm_debug_messages" ];
  environment.etc."systemd/system-sleep/unlock-after-hibernate".source =
    pkgs.writeShellScript "unlock-after-hibernate" ''
      [ "$1" = post ] && [ "$SYSTEMD_SLEEP_ACTION" = hibernate ] || exit 0
      ${pkgs.util-linux}/bin/dmesg \
        | ${pkgs.gawk}/bin/awk '/hibernation entry/ { ok = 0 } /Hibernation image restored successfully/ { ok = 1 } END { exit !ok }' \
        && ${pkgs.systemd}/bin/loginctl unlock-sessions
    '';
  # btintel_pcie times out entering D2 and returns EBUSY, aborting suspend
  environment.etc."systemd/system-sleep/btintel-pcie".source =
    pkgs.writeShellScript "btintel-pcie" ''
      case "$1" in
        pre) ${pkgs.kmod}/bin/modprobe -r btintel_pcie ;;
        post) ${pkgs.kmod}/bin/modprobe btintel_pcie ;;
      esac
    '';
  systemd.services.caps-to-escape = {
    description = "Remap caps lock to escape in the EC";
    wantedBy = [
      "multi-user.target"
      "suspend.target"
      "hibernate.target"
      "suspend-then-hibernate.target"
    ];
    after = [
      "suspend.target"
      "hibernate.target"
      "suspend-then-hibernate.target"
    ];
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${pkgs.framework-tool}/bin/framework_tool --remap-key 4 4 0x0076";
    };
  };

  security.pam.services.swaylock.rules.auth.fprintd.order =
    config.security.pam.services.swaylock.rules.auth.unix.order + 10;
}
