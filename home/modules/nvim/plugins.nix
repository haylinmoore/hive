{ pkgs, ... }:

let
  inherit (pkgs) fetchFromGitHub;
  inherit (pkgs.vimUtils) buildVimPlugin;
in
{
  nvim-treesitter =
    (buildVimPlugin {
      pname = "nvim-treesitter";
      version = "2025-09-28";
      src = fetchFromGitHub {
        owner = "nvim-treesitter";
        repo = "nvim-treesitter";
        rev = "77362027f7aa850c87419fd571151e76b0b342a6";
        sha256 = "117qp8789qixg4xs5f50ip3fkdn50m233z9syxdijslw80qgy180";
      };
      nvimSkipModules = [
        "nvim-treesitter._meta.parsers"
      ];
      meta.homepage = "https://github.com/nvim-treesitter/nvim-treesitter/";
      meta.hydraPlatforms = [ ];
    }).overrideAttrs
      (oldAttrs: {
        passthru.withPlugins = pkgs.vimPlugins.nvim-treesitter.withPlugins;
      });
}
