{
  pkgsi686Linux,
  stdenv,
  dpkg,
  buildFHSEnv,
  writeShellScript,
  lib,
  fetchurl,
  fetchFromGitHub,
  ...
}:

pkgsi686Linux.stdenv.mkDerivation rec {
  pname = "pangox-compat";
  version = "0.0.2";

  src = fetchurl {
    url = "mirror://gnome/sources/${pname}/0.0/${pname}-${version}.tar.xz";
    sha256 = "0ip0ziys6mrqqmz4n71ays0kf5cs1xflj1gfpvs4fgy2nsrr482m";
  };

  preConfigure = "./autogen.sh";

  postConfigure = ''
    substituteInPlace ./pangox.c \
    --replace-fail 'font_class->find_shaper = pango_x_font_find_shaper;' ' ' 
  '';

  env.NIX_CFLAGS_COMPILE = toString [
    "-Wno-error=deprecated-declarations"
    "-Wno-error=incompatible-pointer-types"
    "-Wno-error=int-conversion"
    "-Wno-error=implicit-function-declaration"
  ];

  nativeBuildInputs = (
    with pkgsi686Linux;
    [
      pkg-config
      pango
      autoconf
      automake
      libtool
      which
    ]
  );
  buildInputs = (
    with pkgsi686Linux;
    [
      glib
      pango
      libX11
    ]
  );
}
