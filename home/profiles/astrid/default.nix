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

  home.stateVersion = "25.11";
  home.username = "haylin";
  home.homeDirectory = "/home/haylin";

  # astrid uses JuliaMono
  programs.kitty.settings = {
    font_family = "JuliaMono";
    bold_font = "JuliaMono Bold";
    bold_italic_font = "JuliaMono Bold Italic";
    italic_font = "JuliaMono Italic";
  };
}
