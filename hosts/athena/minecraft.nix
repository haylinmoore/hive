{
  config,
  lib,
  pkgs,
  ...
}:
let
  # Server is off, world is kept. Flip to true to bring it back.
  enable = false;
in
{
  # Declared regardless of enable so the world stays persisted and owned
  # rather than sitting in /persistent unreferenced.
  environment.persistence."/persistent".directories = [
    "/var/lib/minecraft"
  ];

  services.minecraft-server = {
    inherit enable;
    eula = true;
    declarative = true;
    openFirewall = true;

    whitelist = {
      haylin = "cb4fbf5b-a7fc-4eb8-9f08-f245afa62898";
      "1sreal" = "bcbd6cc3-bddb-4334-a3ed-18af305cc162";
    };

    serverProperties = {
      server-port = 25565;
      motd = "mrow mrow";
      white-list = true;
      difficulty = "normal";
      gamemode = "survival";
      max-players = 10;
    };
  };
}
