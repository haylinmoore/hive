{ pkgs, ... }:
{
  boot.loader.systemd-boot = {
    enable = true;
    editor = false;
  };
  boot.loader.efi.canTouchEfiVariables = true;
  boot.initrd.systemd.enable = true;

  hardware.enableRedistributableFirmware = true;
  hardware.bluetooth.enable = true;
  zramSwap.enable = true;

  networking.networkmanager.enable = true;
  networking.firewall.enable = true;
  time.timeZone = "America/Los_Angeles";
  i18n.defaultLocale = "en_US.UTF-8";

  services.desktopManager.plasma6.enable = true;
  services.displayManager.sddm = {
    enable = true;
    wayland.enable = true;
  };

  services.pipewire = {
    enable = true;
    alsa.enable = true;
    pulse.enable = true;
  };
  services.printing.enable = true;
  services.fwupd.enable = true;

  programs.firefox.enable = true;
  environment.systemPackages = with pkgs; [
    kdePackages.discover
    kdePackages.kdialog
    libreoffice-qt
    vlc
  ];
}
