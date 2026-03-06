{
  hive,
  pkgsi686Linux,
  stdenv,
  buildFHSEnv,
  writeShellScript,
  fetchurl,
  ...
}:

let

  navigator-dist = stdenv.mkDerivation {
    pname = "netscape-navigator-dist";
    version = "9.0.0.5";
    src = fetchurl {
      url = "https://www.zx.net.nz/mirror/http.netscape.com.edgesuite.net/pub/netscape9/en-US/9.0/unix/linux/netscape-navigator-9.0.0.5.tar.gz";
      sha256 = "sha256-pxja4JLsF3z1cJbUEy6PybPPXdIg7HMaq9aTqM5aSAI=";
    };
    sourceRoot = "navigator";
    dontBuild = true;
    installPhase = ''
      mkdir -p $out
      cp -r . $out/
      chmod +x $out/navigator $out/navigator-bin $out/run-mozilla.sh
    '';
  };
in
buildFHSEnv {
  name = "netscape-navigator";
  targetPkgs =
    pkgs:
    (
      with pkgsi686Linux;
      [
        glibc
        gtk2
        atk
        gdk-pixbuf
        pango
        glib
        libx11
        libxrender
        fontconfig
        freetype
        libxt
        libxft
        libz
      ]
      ++ [
        hive.pkgs.pangox32
        hive.pkgs.libstdcpp5
      ]
    );
  runScript = writeShellScript "run-navigator" ''
    export LD_LIBRARY_PATH=/usr/lib32''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
    cd ${navigator-dist}
    exec ./navigator "$@"
  '';
}
