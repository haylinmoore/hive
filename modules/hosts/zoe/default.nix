{
  deployment.targetHost = "zoe.infra.hayl.in";

  imports = [
    ./hardware-configuration.nix
    ./networking.nix
    ./services.nix
  ];

  networking.firewall.enable = false;

  sops.secrets."duckdns" = {
    sopsFile = ../../../secrets/zoe/tokens.yaml;
    key = "duckdns";
  };
}
