{ pkgs, ... }:

pkgs.stdenv.mkDerivation {
  pname = "lambda";
  version = "1.0";

  src = ./.;

  nativeBuildInputs = [ pkgs.go ];

  env = {
    GOOS = "js";
    GOARCH = "wasm";
    CGO_ENABLED = "0";
    GOFLAGS = "-trimpath";
  };

  buildPhase = ''
    export HOME=$TMPDIR GOCACHE=$TMPDIR/go-cache
    go build -ldflags="-s -w" -o sheepda.wasm .
  '';

  installPhase = ''
    mkdir -p $out
    cp -r index.html main.js shp sheepda.wasm $out/
    cp ${pkgs.go}/share/go/lib/wasm/wasm_exec.js $out/
  '';
}
