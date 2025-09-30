{ pkgs, hive, ... }:

let
  treefmt-nix = import hive.sources.treefmt-nix;
in
treefmt-nix.mkWrapper pkgs {
  projectRootFile = ".git/config";
  programs = {
    nixfmt.enable = true;
    shellcheck.enable = true;
    jsonfmt.enable = true;
    yamlfmt.enable = true;
    mdformat.enable = true;
  };
  settings.global.excludes = [
    "web/www/content/words/*"
  ];
}
