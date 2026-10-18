let
  hive = import ./default.nix;
  pkgs = hive.nixpkgs;
in
pkgs.mkShell {
  buildInputs = with pkgs; [
    colmena
    dnscontrol
    sops
    ssh-to-age
    hive.tools.treefmt
  ];
}
