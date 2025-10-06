{
  lib,
  stdenv,
  systemd,
  autoPatchelfHook,
  makeWrapper,
  wayland,
  libxkbcommon,
  requireFile,
  ...
}:

stdenv.mkDerivation rec {
  pname = "cobalt";
  version = "2025-09-29";

  src = requireFile rec {
    name = "cobalt";
    sha256 = "1vs2mxdgkywi61kw3vckwhapslyiyaldilv85iflg1rsffrqkqja";
    message = ''
      https://derelict-engineering.itch.io/cobalt, buy and download the linux version.
      Upload the cobalt executable to your nix-store.
      nix-prefetch-url --type sha256 file://\$PWD/${name}
    '';
  };

  nativeBuildInputs = [
    autoPatchelfHook
    makeWrapper
  ];

  buildInputs = [
    systemd
    wayland
    libxkbcommon
  ];

  dontUnpack = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    cp $src $out/bin/cobalt
    chmod +x $out/bin/cobalt

    runHook postInstall
  '';

  postFixup = ''
    wrapProgram $out/bin/cobalt \
      --prefix LD_LIBRARY_PATH : "${
        lib.makeLibraryPath [
          wayland
          libxkbcommon
        ]
      }"
  '';

  meta = with lib; {
    description = "Cobalt";
    homepage = "https://derelict-engineering.itch.io/cobalt";
    license = licenses.unfree;
    platforms = platforms.linux;
    maintainers = [ ];
  };
}
