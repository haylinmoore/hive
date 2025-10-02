{
  lib,
  stdenv,
  alsa-lib,
  systemd,
  autoPatchelfHook,
  xorg,
  makeWrapper,
  mesa,
  libGL,
  vulkan-loader,
  addDriverRunpath,
  requireFile,
  ...
}:

stdenv.mkDerivation rec {
  pname = "tunnet";
  version = "27";

  src = requireFile rec {
    name = "tunnet";
    sha256 = "0m23da8nd9w1lbngx4dmn1bf47nfmmav0x3zbxlv7lai9k6hhdcs";
    message = ''
      https://puzzled-squid.itch.io/tunnet, buy and download the linux version.
      Unzip into a folder and upload the linux executable to your nix-store.
      nix-prefetch-url --type sha256 file://\$PWD/${name}
    '';
  };

  nativeBuildInputs = [
    autoPatchelfHook
    makeWrapper
    addDriverRunpath
  ];

  buildInputs = [
    alsa-lib
    systemd
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXext
    mesa
    libGL
    vulkan-loader
  ];

  runtimeDependencies = [
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libXext
    mesa
    libGL
    vulkan-loader
  ];
  dontUnpack = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/bin
    cp $src $out/bin/tunnet
    chmod +x $out/bin/tunnet

    runHook postInstall
  '';

  postFixup = ''
    addDriverRunpath $out/bin/tunnet
    wrapProgram $out/bin/tunnet \
      --prefix LD_LIBRARY_PATH : "${
        lib.makeLibraryPath [
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libXext
          mesa
          libGL
          vulkan-loader
        ]
      }"
  '';

  meta = with lib; {
    description = "Tunnet";
    homepage = "https://puzzled-squid.itch.io/tunnet";
    license = licenses.unfree;
    platforms = platforms.linux;
    maintainers = [ ];
  };
}
