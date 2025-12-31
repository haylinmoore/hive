{ pkgs, ... }:

pkgs.stdenv.mkDerivation rec {
  pname = "aconite";
  version = "1.0";

  src = ./.;

  buildPhase = ''
    mkdir -p $out
    cp -r ${src}/* $out/
  '';
}
