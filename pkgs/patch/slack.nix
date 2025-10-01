{
  pkgs,
  makeWrapper,
  symlinkJoin,
  ...
}:

symlinkJoin {
  name = "slack-wayland";
  paths = [ pkgs.slack ];
  buildInputs = [ makeWrapper ];
  postBuild = ''
    wrapProgram $out/bin/slack \
      --add-flags "--ozone-platform=wayland"
  '';
}
