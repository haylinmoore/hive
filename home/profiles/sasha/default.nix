{
  pkgs,
  lib,
  hive,
  ...
}:
{
  imports = [
    ../../modules/haylin/base.nix
    ../../modules/haylin/gui.nix
    ./wm.nix
  ];

  home.stateVersion = "24.11";
  home.username = "haylin";
  home.homeDirectory = "/home/haylin";

  # sasha-specific packages
  home.packages = with pkgs; [
    virtualgl
    discord
    spotify
    signal-desktop
    gimp
    tor-browser
    russ
    nicotine-plus
    gapless
    finamp

    # memes
    hyfetch

    # code
    kubectl
    gnumake
    ccls
    gdb
    ncdu

    # games
    prismlauncher

    # golang
    go
    gopls
    gotools
    go-tools
    delve
  ];

  # sasha uses Berkeley Mono
  programs.kitty.settings = {
    font_family = "Berkeley Mono";
    bold_font = "Berkeley Mono Bold";
    bold_italic_font = "Berkeley Mono Bold Italic";
    italic_font = "Berkeley Mono Italic";
  };
}
