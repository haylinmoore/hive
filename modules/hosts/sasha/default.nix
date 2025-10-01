{
  config,
  pkgs,
  lib,
  hive,
  ...
}:
{
  imports = [
    ./hardware-configuration.nix
    ./platform.nix
    ./wm.nix
    ./unbound.nix
    ./fonts.nix
    ./services.nix
    ../../shared/nix.nix
  ];

  nixpkgs.config.allowUnfree = true;

  system.stateVersion = "24.11";

  # Bootloader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  security.sudo.wheelNeedsPassword = false;

  networking.hostName = "sasha";
  networking.nameservers = [ "127.0.0.1" ];

  networking.networkmanager = {
    enable = true;
    plugins = [
      pkgs.networkmanager-l2tp
      pkgs.networkmanager_strongswan
    ];
  };

  time.timeZone = "US/Pacific";

  # Locale settings
  i18n.defaultLocale = "en_US.UTF-8";
  i18n.extraLocaleSettings = {
    LC_ADDRESS = "en_US.UTF-8";
    LC_IDENTIFICATION = "en_US.UTF-8";
    LC_MEASUREMENT = "en_US.UTF-8";
    LC_MONETARY = "en_US.UTF-8";
    LC_NAME = "en_US.UTF-8";
    LC_NUMERIC = "en_US.UTF-8";
    LC_PAPER = "en_US.UTF-8";
    LC_TELEPHONE = "en_US.UTF-8";
    LC_TIME = "en_US.UTF-8";
  };

  # Hardware
  hardware = {
    graphics.enable = true;
    graphics.extraPackages = with pkgs; [
      intel-media-driver
      intel-vaapi-driver
      libvdpau-va-gl
    ];
    bluetooth.enable = true;
  };

  # System packages
  environment.systemPackages = with pkgs; [
    vim
    wget
    curl
    libva
    libva-utils
    networkmanager-l2tp
    networkmanagerapplet
    podman-compose
  ];

  # Networking/Firewall
  networking.firewall.enable = true;
  networking.firewall.allowPing = true;
  networking.firewall.allowedTCPPorts = [
    8000
    8080
    3000
  ];

  # Virtualization
  virtualisation.podman = {
    enable = true;
    dockerCompat = true;
    defaultNetwork.settings.dns_enabled = true;
  };

  # Programs
  programs.wireshark = {
    enable = true;
    usbmon.enable = true;
    dumpcap.enable = true;
  };
  programs.zsh.enable = true;
  programs.ssh.startAgent = true;
  programs.nix-ld.enable = true;
  programs.steam = {
    enable = true;
    remotePlay.openFirewall = true;
    dedicatedServer.openFirewall = true;
    localNetworkGameTransfers.openFirewall = true;
  };

  # User configuration
  users.users.haylin = {
    isNormalUser = true;
    description = "haylin";
    extraGroups = [
      "dialout"
      "audio"
      "networkmanager"
      "wheel"
      "podman"
      "wireshark"
    ];
    shell = pkgs.zsh;
  };
  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    extraSpecialArgs = { inherit hive; };
    users.haylin = import ../../../home/profiles/sasha;
  };

  # Environment variables
  environment.sessionVariables.TERMINAL = [ "kitty" ];
  environment.sessionVariables.NIXOS_OZONE_WL = "1";
  environment.sessionVariables.LIBVA_DRIVER_NAME = "iHD";

  # Documentation
  documentation.enable = true;
  documentation.man.enable = true;
  documentation.dev.enable = true;
}
