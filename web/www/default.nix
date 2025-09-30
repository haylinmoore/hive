{ pkgs, hive, ... }:

let
  naersk = pkgs.callPackage hive.sources.naersk { };
in
naersk.buildPackage {
  src = ./.;

  nativeBuildInputs = with pkgs; [
    image_optim
  ];

  postInstall = ''
    mkdir -p $out/assets
    mkdir -p $out/content
    cp -r ./assets/* $out/assets/

    image_optim -r $out/assets/

    cp -r ./content/* $out/content/
  '';
}
