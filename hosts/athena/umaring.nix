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
        proxyPass = "https://umaring.github.io";
        extraConfig = ''
          proxy_ssl_server_name on;
          proxy_set_header Host umaring.github.io;
        '';
      };
    };
  };
}
