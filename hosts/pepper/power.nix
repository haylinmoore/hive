{ pkgs, ... }:
{
  services.power-profiles-daemon.enable = true;

  systemd.services.power-profile-auto = {
    description = "Pick the power profile for the current power source";
    after = [ "power-profiles-daemon.service" ];
    requires = [ "power-profiles-daemon.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig.Type = "oneshot";
    script = ''
      if [ "$(cat /sys/class/power_supply/AC/online)" = 1 ]; then
        ${pkgs.power-profiles-daemon}/bin/powerprofilesctl set balanced
      else
        ${pkgs.power-profiles-daemon}/bin/powerprofilesctl set power-saver
      fi
    '';
  };

  services.udev.extraRules = ''
    SUBSYSTEM=="power_supply", KERNEL=="AC", TAG+="systemd", ENV{SYSTEMD_WANTS}="power-profile-auto.service"
    SUBSYSTEM=="pci", ACTION=="add", ATTR{power/control}="auto"
  '';

  boot.kernel.sysctl = {
    "kernel.nmi_watchdog" = 0;
    "vm.dirty_writeback_centisecs" = 1500;
  };

  environment.systemPackages = [ pkgs.powertop ];
}
