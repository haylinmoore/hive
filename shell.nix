let
  mono = import ./default.nix;
  pkgs = mono.pkgs;
  treefmt-nix = import mono.sources.treefmt-nix;

  treefmt = treefmt-nix.mkWrapper pkgs {
    projectRootFile = ".git/config";
    programs = {
      nixfmt.enable = true;
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
