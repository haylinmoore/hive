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
    ./fonts.nix
    ./services.nix
    ./wemux.nix
    ../../nixos/shared/nix.nix
  ];

  system.stateVersion = "25.11";

  # Bootloader
  boot.loader.systemd-boot.enable = true;
  boot.loader.efi.canTouchEfiVariables = true;

  security.sudo.wheelNeedsPassword = false;

  networking.hostName = "astrid";

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
    bluetooth.enable = true;
  };

  # System packages
  environment.systemPackages = with pkgs; [
    hello
    vim
    wget
    curl
    networkmanager-l2tp
    networkmanagerapplet
    podman-compose
    nfs-utils
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
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop"
    ];
  };

  users.users.hmoore = {
    isNormalUser = true;
    description = "hmoore";
    extraGroups = [
      "wheel"
      "podman"
    ];
    shell = pkgs.zsh;
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop"
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILXL2etr9int91HoWiyh8P4+UJH/nb2i5KSJ+tBb+BDi hmoore@qumulo.com"
    ];
  };

  home-manager = {
    useGlobalPkgs = true;
    useUserPackages = true;
    extraSpecialArgs = { inherit hive; };
    users.haylin = import ../../home/profiles/astrid;
    users.hmoore = import ../../home/profiles/astrid-hmoore;
  };

  # Environment variables
  environment.sessionVariables.TERMINAL = [ "kitty" ];
  environment.sessionVariables.NIXOS_OZONE_WL = "1";

  # Documentation
  documentation.enable = true;
  documentation.man.enable = true;
  documentation.dev.enable = true;
}
