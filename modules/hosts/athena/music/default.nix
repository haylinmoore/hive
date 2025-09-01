{
  config,
  lib,
  pkgs,
  ...
}:

let
  musicLandingPage = pkgs.writeTextFile {
    name = "music-landing-page";
    text = builtins.readFile ./index.html;
    destination = "/index.html";
  };
  usersConfig = import ./users-config.nix;
in
{
  imports = [
    ./navidrome.nix
    # ./jellyfin.nix
    # ./slskd.nix
    ./feishin.nix
    ./upload-container.nix
  ];

  users.users.haylin.extraGroups = [ "media" ];

  users.groups.media = {
    gid = usersConfig.groups.media.gid;
  };

  services.nginx = {
    enable = true;
    virtualHosts."music.hayl.in" = {
      forceSSL = true;
      useACMEHost = "hayl.in";
      locations."/".root = musicLandingPage;
    };
  };
}
