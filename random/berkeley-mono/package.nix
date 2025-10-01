{
  lib,
  requireFile,
  stdenvNoCC,
  unzip,
  variant ? "tff-ligatureson-0variant1-7variant0",
  sha ? "0fkhbgddlpjh6dwb7mc3hk0qhv8skvx367z911bvkhi5zhqw94jp",
  ...
}:

stdenvNoCC.mkDerivation (finalAttrs: {
  pname = "berkeley-mono";
  version = "2.002";

  src = requireFile rec {
    name = "${finalAttrs.pname}-${variant}-${finalAttrs.version}.zip";
    sha256 = "${sha}";
    message = ''
      This file needs to be manually downloaded from the Berkeley Graphics
      site (https://berkeleygraphics.com/accounts). An email will be sent to
      get a download link.

      Select the variant that matches "${variant}"
      & download the zip file.

      Then run:

      mv \$PWD/berkeley-mono-typeface.zip \$PWD/${name}
      nix-prefetch-url --type sha256 file://\$PWD/${name}
    '';
  };

  outputs = [
    "out"
    "web"
  ];

  nativeBuildInputs = [
    unzip
  ];

  unpackPhase = ''
    unzip $src
  '';

  installPhase = ''
    runHook preInstall

    #install -D -m444 -t $out/share/fonts/opentype */*.otf
    mkdir -p $out/share/fonts/truetype/
    mkdir -p $web/share/fonts/webfonts/
    mkdir -p $out/share/fonts/opentype/
    find . -name "*.ttf" -exec install -D -m444 {} $out/share/fonts/truetype/ \;
    find . -name "*.woff2" -exec install -D -m444 {} $web/share/fonts/webfonts \;
    find . -name "*.otf" -exec install -D m444 {} $out/share/fonts/opentype \;
    #install -D -m444 -t $out/share/fonts/truetype */*.ttf
    #install -D -m444 -t $web/share/fonts/webfonts berkeley-mono/*/*.woff2
    #install -D -m444 -t $variable/share/fonts/truetype */*.ttf
    #install -D -m444 -t $variableweb/share/fonts/webfonts berkeley-mono-variable/WEB/*.woff2

    runHook postInstall
  '';

  meta = {
    description = "Berkeley Mono Typeface";
    longDescription = "…";
    homepage = "https://berkeleygraphics.com/typefaces/berkeley-mono";
    license = lib.licenses.unfree;
    platforms = lib.platforms.all;
  };
})
