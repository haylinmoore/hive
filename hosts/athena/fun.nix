{
  config,
  lib,
  pkgs,
  ...
}:

let
  # Common authentik forward auth configuration
  authentikAuthConfig = ''
    # Increase buffer size for large headers
    proxy_buffers 8 16k;
    proxy_buffer_size 32k;

    # Support for websocket
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection $connection_upgrade_keepalive;

    # authentik forward auth
    auth_request /outpost.goauthentik.io/auth/nginx;
    error_page 401 = @goauthentik_proxy_signin;
    auth_request_set $auth_cookie $upstream_http_set_cookie;
    add_header Set-Cookie $auth_cookie;

    # Translate headers from the outpost back to the actual upstream
    auth_request_set $authentik_username $upstream_http_x_authentik_username;
    auth_request_set $authentik_groups $upstream_http_x_authentik_groups;
    auth_request_set $authentik_entitlements $upstream_http_x_authentik_entitlements;
    auth_request_set $authentik_email $upstream_http_x_authentik_email;
    auth_request_set $authentik_name $upstream_http_x_authentik_name;
    auth_request_set $authentik_uid $upstream_http_x_authentik_uid;

    proxy_set_header X-authentik-username $authentik_username;
    proxy_set_header X-authentik-groups $authentik_groups;
    proxy_set_header X-authentik-entitlements $authentik_entitlements;
    proxy_set_header X-authentik-email $authentik_email;
    proxy_set_header X-authentik-name $authentik_name;
    proxy_set_header X-authentik-uid $authentik_uid;
  '';
in
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
        extraConfig = authentikAuthConfig;
      };

      # ruTorrent web interface
      locations."/rutorrent/" = {
        proxyPass = "http://127.0.0.1:8090/";
        extraConfig = authentikAuthConfig;
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
        extraConfig = authentikAuthConfig;
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
