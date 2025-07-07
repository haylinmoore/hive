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
in
{
  imports = [
    ./navidrome.nix
    ./jellyfin.nix
    ./slskd.nix
    ./feishin.nix
  ];

  users.users.alice = {
    isNormalUser = true;
    createHome = true;
    extraGroups = [ "media" ];
    openssh.authorizedKeys.keys = [
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOdK5ssxU1XL5iOOJjQ27Plo4nFmS6df9GhkOYg1GJaT"
    ];
  };
  users.users.haylin.extraGroups = [ "media" ];

  users.groups.media = { };

  services.nginx = {
    enable = true;
    virtualHosts."music.hayl.in" = {
      forceSSL = true;
      enableACME = true;
      locations."/".root = musicLandingPage;
    };
  };
}
