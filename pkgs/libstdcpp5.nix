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
    url = "https://archive.debian.org/debian/pool/main/g/gcc-3.3/libstdc++5_3.3.6-32_i386.deb";
    sha256 = "sha256-6lIy8C5wjsGHbVU5rJW2QHrqCJVPWRSqVI31xm+mIpQ=";
  };

  unpackPhase = ''
    ${dpkg}/bin/dpkg -x $src .
  '';

  installPhase = ''
    mkdir -p $out/lib
    cp ./usr/lib/*/*.so.5 $out/lib
  '';
}
