# Personal/home neovim configuration
{ config, pkgs, ... }:
{
  imports = [
    ./plugins.nix
  ];

  programs.neovim = {
    defaultEditor = true;
    enable = true;
    extraConfig = builtins.readFile ./shared.vim;
    extraPackages = [ pkgs.fzf ];
    vimAlias = true;
    vimdiffAlias = true;
  };

  programs.fzf = {
    enable = true;
    defaultCommand = "${pkgs.fd}/bin/fd --type f";
    fileWidgetCommand = "${pkgs.fd}/bin/fd --type f";
  };
}
