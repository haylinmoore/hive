{ pkgs, ... }:
{
  boot.loader.systemd-boot = {
    enable = true;
    editor = false;
  };
  boot.loader.efi.canTouchEfiVariables = true;

  hardware.enableRedistributableFirmware = true;
  hardware.bluetooth.enable = true;
  zramSwap.enable = true;

  networking.networkmanager.enable = true;
  networking.networkmanager.ensureProfiles.profiles.devhack = {
    connection = {
      id = "wlan0";
      type = "wifi";
    };
    wifi = {
      ssid = "wlan0";
      mode = "infrastructure";
    };
    wifi-security = {
      key-mgmt = "wpa-psk";
      psk = "/dev/hack";
    };
    ipv4.method = "auto";
    ipv6.method = "auto";
  };
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
