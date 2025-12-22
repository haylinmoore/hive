{
  lib,
  stdenv,
  fetchFromGitHub,
  php,
  git,
  makeWrapper,
  ...
}:

stdenv.mkDerivation rec {
  pname = "arcanist";
  version = "unstable-2025-12-19";

  src = fetchFromGitHub {
    owner = "phorgeit";
    repo = "arcanist";
    rev = "52b17b256d1348628f6b2566e8dbe82d57f7fb93";
    hash = "sha256-MUH9ikq3/J+Qu7tWXqJjKgFHEnj0Y+CrGhULmg6TzKg=";
  };

  buildInputs = [ php ];
  nativeBuildInputs = [ makeWrapper ];

  installPhase = ''
    runHook preInstall

    mkdir -p $out/lib/arcanist
    cp -r . $out/lib/arcanist

    mkdir -p $out/bin
    makeWrapper ${php}/bin/php $out/bin/arc \
      --add-flags "$out/lib/arcanist/bin/arc" \
      --prefix PATH : ${
        lib.makeBinPath [
          git
          php
        ]
      }

    runHook postInstall
  '';

  meta = with lib; {
    description = "Command-line interface to Phorge/Phabricator";
    homepage = "https://we.phorge.it/book/phorge/article/arcanist_quick_start/";
    license = licenses.asl20;
    platforms = platforms.unix;
    mainProgram = "arc";
  };
}
