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

    # Add personal-specific plugins here if needed
    # plugins = with pkgs.vimPlugins; [
    #   # personal-specific plugins
    # ];
  };
}
