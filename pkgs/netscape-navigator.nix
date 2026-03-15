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

  versions = {
    v9005 = builtins.fetchTarball {
      url = "https://archive.hayl.in/netscape-navigator/netscape-navigator-9.0.0.5.tar.gz";
      sha256 = "0ms64in12lh9sjbk4n77fcagandiwi8gm6qd9xf2bnbyddn3hqpy";
    };
    v9006 = builtins.fetchTarball {
      url = "https://archive.hayl.in/netscape-navigator/netscape-navigator-9.0.0.6.tar.gz";
      sha256 = "0fq57w55i0jsa26j4b7cadzbkb8hg8j2d4228jngqmkkn8ldbwfl";
    };
  };

in
builtins.mapAttrs (
  name: src:
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
      cd ${src}
      exec ./navigator "$@"
    '';
  }
) versions
