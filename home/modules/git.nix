{
  config,
  pkgs,
  lib,
  ...
}:

{
  programs.git = {
    enable = true;
    userName = "Haylin Moore";
    extraConfig = {
      init.defaultBranch = "main";
      pull = {
        autosetupremote = true;
        rebase = true;
      };
      push = {
        autoSetupRemote = true;
      };
    };
  };
}
