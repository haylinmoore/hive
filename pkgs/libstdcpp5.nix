{
  stdenv,
  dpkg,
  fetchurl,
  ...
}:

stdenv.mkDerivation {
  pname = "libstdc++5";
  version = "3.3.6";
  src = fetchurl {
    url = "https://archive.debian.org/debian/pool/main/g/gcc-3.3/libstdc++5_3.3.6-15_i386.deb";
    sha256 = "sha256-NCQwRGYjK7XHufw10t6Pu7go2LPv4Y1RIVjpAuIYhDI=";
  };
  nativeBuildInputs = [ dpkg ];
  dontUnpack = true;
  installPhase = ''
    mkdir -p $out/lib
    dpkg-deb -x $src unpacked
    find unpacked -name '*.so*' -exec cp -a {} $out/lib/ \;
  '';
}
