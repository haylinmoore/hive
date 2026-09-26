{ config, ... }:
{
  # Services
  services.openssh.enable = false;
  services.upower.enable = true;
  services.power-profiles-daemon.enable = true;
  services.printing.enable = true;
  services.lldpd.enable = true;
  services.tailscale.enable = true;
  services.strongswan.enable = true;
  services.fprintd.enable = true;
  services.logind.settings.Login.HandleLidSwitch = "suspend-then-hibernate";
  systemd.sleep.settings.Sleep.HibernateDelaySec = "1h";
  security.pam.services.swaylock.rules.auth.fprintd.order =
    config.security.pam.services.swaylock.rules.auth.unix.order + 10;
}
