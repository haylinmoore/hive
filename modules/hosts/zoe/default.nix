{
  deployment.targetHost = "zoe.infra.hayl.in";

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ./services.nix
    ./secrets.nix
  ];

  networking.firewall.enable = false;
}
