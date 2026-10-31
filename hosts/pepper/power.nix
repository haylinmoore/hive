{ pkgs, ... }:
{
  services.power-profiles-daemon.enable = true;

  services.udev.extraRules = ''
    SUBSYSTEM=="pci", ACTION=="add", ATTR{power/control}="auto"
  '';

  boot.extraModprobeConfig = "options snd_hda_intel power_save=1";

  boot.kernel.sysctl = {
    "kernel.nmi_watchdog" = 0;
    "vm.dirty_writeback_centisecs" = 1500;
  };

  systemd.coredump.settings.Coredump = {
    Storage = "none";
    ProcessSizeMax = 0;
  };

  environment.systemPackages = [
    pkgs.powertop
    pkgs.framework-tool
  ];
}
