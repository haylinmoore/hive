{ 
  pkgsi686Linux,
  stdenv,
  dpkg,
  buildFHSEnv,
  writeShellScript,
  fetchurl,
  ...
}:

let
  pangox-stub = pkgsi686Linux.stdenv.mkDerivation {
    pname = "pangox-stub";
    version = "0.1";
    dontUnpack = true;
    buildPhase = ''
      cat > pangox-stub.c << 'EOF'
      void pango_x_font_map_for_display(void) {}
      void pango_x_shutdown_display(void) {}
      void pango_x_font_map_get_font_cache(void) {}
      void pango_x_font_subfont_xlfd(void) {}
      EOF
      gcc -shared -o libpangox-1.0.so.0 pangox-stub.c -Wl,-soname,libpangox-1.0.so.0
    '';
    installPhase = ''
      mkdir -p $out/lib
      cp libpangox-1.0.so.0 $out/lib/
      ln -s libpangox-1.0.so.0 $out/lib/libpangox-1.0.so
    '';
  };

  libstdcpp5 = stdenv.mkDerivation {
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
  };

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
  targetPkgs = pkgs: (with pkgsi686Linux; [
    glibc
    gcc-unwrapped.lib
    gtk2
    glib
    pango
    cairo
    atk
    gdk-pixbuf
    fontconfig
    freetype
    libx11
    libxt
    libxext
    libxrender
    libxinerama
    libxcursor
    libxrandr
    libxcomposite
    libxdamage
    libxfixes
    libsm
    libice
    libxp
    libxmu
    libxpm
    libxaw
    libxft
    zlib
    libpng
    alsa-lib
    dbus
    dbus-glib
  ]) ++ [ pangox-stub libstdcpp5 ];
  runScript = writeShellScript "run-navigator" ''
    export LD_LIBRARY_PATH=/usr/lib32''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}
    cd ${navigator-dist}
    exec ./navigator "$@"
  '';
}
