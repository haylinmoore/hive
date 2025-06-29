{
  deployment.targetHost = "bella.infra.hayl.in";

  networking.firewall.enable = true;
  networking.firewall.allowedTCPPorts = [
    80
    443
    2222
  ];

  imports = [
    ../../shared/pve.nix
    ./hardware-configuration.nix
    ./networking.nix
    ./services.nix
    ./secrets.nix
  ];
}
