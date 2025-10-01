{
  lib,
  stdenv,
  fetchurl,
  ...
}:

stdenv.mkDerivation rec {
  pname = "alsa-ucm-conf";
  version = "1.2.14";

  src = fetchurl {
    url = "mirror://alsa/lib/alsa-ucm-conf-${version}.tar.bz2";
    hash = "sha256-MumAn1ktkrl4qhAy41KTwzuNDx7Edfk3Aiw+6aMGnCE=";
  };

  dontBuild = true;

  installPhase = ''
    runHook preInstall

    mkdir -p $out/share/alsa
    cp -r ucm ucm2 $out/share/alsa

    runHook postInstall
  '';

  meta = with lib; {
    homepage = "https://alsa-project.org/";
    description = "ALSA Use Case Manager configuration";
    longDescription = ''
      The Advanced Linux Sound Architecture (ALSA) Use Case Manager (UCM)
      configuration contains the configuration files and profiles for the
      ALSA UCM.
    '';
    license = licenses.bsd3;
    platforms = platforms.linux;
    maintainers = with maintainers; [ pSub ];
  };
}
