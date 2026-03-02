{
  config,
  lib,
  pkgs,
  ...
}:

{
  # required by mautrix-discord for end-to-bridge encryption
  nixpkgs.config.permittedInsecurePackages = [
    "olm-3.2.16"
  ];

  sops.secrets."mautrix-discord" = {
    sopsFile = ../../secrets/athena/mautrix-discord.env;
    format = "dotenv";
    restartUnits = [ "mautrix-discord.service" ];
  };

  environment.persistence."/persistent".directories = [
    "/var/lib/private/continuwuity"
    "/var/lib/mautrix-discord"
  ];

  services.matrix-continuwuity = {
    enable = true;
    settings.global = {
      server_name = "chat.estrogen.coffee";
      address = [ "127.0.0.1" "::1" ];
      port = [ 6167 ];
      allow_registration = true;  # disable after creating admin account
      registration_token = "haylin-setup";
      allow_encryption = true;
      allow_federation = true;
      trusted_servers = [ "matrix.org" ];
    };
  };

  services.mautrix-discord = {
    enable = true;
    environmentFile = "/run/secrets/mautrix-discord";
    registerToSynapse = false;
    serviceDependencies = [
      "mautrix-discord-registration.service"
      "continuwuity.service"
    ];
    settings = {
      homeserver = {
        address = "http://localhost:6167";
        domain = "chat.estrogen.coffee";
      };
      bridge.public_address = "https://chat.estrogen.coffee";
      bridge = {
        permissions = {
          "*" = "relay";
          "chat.estrogen.coffee" = "relay";
          "@haylin:catgirl.cloud" = "admin";
          "@admin:chat.estrogen.coffee" = "admin";
        };
        encryption = {
          allow = false;
          default = false;
        };
        relay = {
          enabled = true;
        };
      };
    };
  };

  services.nginx.virtualHosts."chat.estrogen.coffee" = {
    forceSSL = true;
    useACMEHost = "estrogen.coffee";
    locations."/" = {
      proxyPass = "http://[::1]:6167";
      proxyWebsockets = true;
      extraConfig = ''
        client_max_body_size 20M;
      '';
    };
    locations."/mautrix-discord/" = {
      proxyPass = "http://localhost:29334";
    };
    locations."= /.well-known/matrix/client".extraConfig = ''
      add_header Content-Type application/json;
      add_header Access-Control-Allow-Origin *;
      return 200 '{"m.homeserver":{"base_url":"https://chat.estrogen.coffee"}}';
    '';
    locations."= /.well-known/matrix/server".extraConfig = ''
      add_header Content-Type application/json;
      return 200 '{"m.server":"chat.estrogen.coffee:443"}';
    '';
  };
}
