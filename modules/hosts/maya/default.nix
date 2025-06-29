{
  deployment.targetHost = "maya.infra.hayl.in";

  networking.firewall.enable = true;
  networking.firewall.allowedTCPPorts = [
    80
    443
  ];

  imports = [
    ../../shared/pve.nix
    ./hardware-configuration.nix
    ./networking.nix
  ];
}
