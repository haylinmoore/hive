{
  config,
  pkgs,
  lib,
  ...
}:

{
  programs.git = {
    enable = true;
    settings = {
      user.name = "Haylin Moore";
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
