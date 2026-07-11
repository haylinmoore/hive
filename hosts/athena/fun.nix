{
  config,
  lib,
  pkgs,
  hive,
  ...
}:
{
  services.nginx = {
    # Add websocket upgrade map for authentik
    appendHttpConfig = ''
      map $http_upgrade $connection_upgrade_keepalive {
          default upgrade;
          ""      "";
      }
    '';

    virtualHosts."fun.hayl.in" = {
      forceSSL = true;
      useACMEHost = "hayl.in";

      # For now, serve default nginx page
      locations."/" = {
        extraConfig = hive.web.authentik;
      };

      # ruTorrent web interface
      locations."/rutorrent/" = {
        proxyPass = "http://127.0.0.1:8090/";
        extraConfig = hive.web.authentik;
      };

      # Redirect /rutorrent to /rutorrent/
      locations."= /rutorrent" = {
        extraConfig = ''
          return 301 /rutorrent/;
        '';
      };

      # slskd (Soulseek daemon)
      locations."/slskd/" = {
        proxyPass = "http://127.0.0.1:5030/slskd/";
        proxyWebsockets = true;
        extraConfig = hive.web.authentik;
      };

      # Redirect /slskd to /slskd/
      locations."= /slskd" = {
        extraConfig = ''
          return 301 /slskd/;
        '';
      };

      # All requests to /outpost.goauthentik.io must be accessible without authentication
      locations."/outpost.goauthentik.io" = {
        proxyPass = "http://127.0.0.1:9000/outpost.goauthentik.io";
        extraConfig = ''
          proxy_set_header X-Original-URL $scheme://$http_host$request_uri;
          add_header Set-Cookie $auth_cookie;
          auth_request_set $auth_cookie $upstream_http_set_cookie;
          proxy_pass_request_body off;
          proxy_set_header Content-Length "";
        '';
      };

      # Special location for when the /auth endpoint returns a 401
      extraConfig = ''
        location @goauthentik_proxy_signin {
          internal;
          add_header Set-Cookie $auth_cookie;
          return 302 /outpost.goauthentik.io/start?rd=$scheme://$http_host$request_uri;
        }
      '';
    };
  };
}
