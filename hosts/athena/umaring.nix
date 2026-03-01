{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
{
  services.nginx = {
    enable = true;
    virtualHosts."umaring.mkr.cx" = {
      forceSSL = true;
      enableACME = true;
      locations."/" = {
        return = "301 https://umaring.github.io$request_uri";
        extraConfig = ''
          add_header Access-Control-Allow-Origin *;
        '';
      };
    };
  };
}
