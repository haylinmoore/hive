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
  ];

  sops.secrets."dollpublish" = {
    sopsFile = ../../../secrets/bella/dollpublish.json;
    key = "";
    format = "json";
    owner = "dollpublish";
    path = "/home/dollpublish/users.json";
    restartUnits = [ "dollpublish.service" ];
  };

  sops.secrets."slskd" = {
    sopsFile = ../../../secrets/bella/slskd.env;
    key = "";
    format = "dotenv";
    owner = "slskd";
    restartUnits = [ "slskd.service" ];
  };
}
