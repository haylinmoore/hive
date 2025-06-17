let
  sources = import ./npins;
  pkgs = import sources.nixpkgs { };
  treefmt-nix = import sources.treefmt-nix;

  treefmt = treefmt-nix.mkWrapper pkgs {
    projectRootFile = ".git/config";
    programs = {
      nixfmt-rfc-style.enable = true;
      shellcheck.enable = true;
      jsonfmt.enable = true;
      yamlfmt.enable = true;
      mdformat.enable = true;
    };
  };
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    colmena
    sops
    ssh-to-age
    treefmt
  ];
}
