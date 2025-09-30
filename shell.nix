let
  hive = import ./default.nix;
  pkgs = hive.pkgs;
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    colmena
    sops
    ssh-to-age
    hive.tools.treefmt
  ];
}
