{
  config,
  pkgs,
  lib,
  hive,
  ...
}:

{
  imports = [
    ../modules/git.nix
    ../modules/nvim-ng/work.nix
    ../modules/work/zsh.nix
    ../modules/work/tmux.nix
  ];

  home.username = "hmoore";
  home.homeDirectory = "/home/hmoore";
  home.stateVersion = "25.05";

  # Work-specific git email
  programs.git.userEmail = "hmoore@qumulo.com";

  nixpkgs.config.allowUnfreePredicate =
    pkg:
    builtins.elem (lib.getName pkg) [
      "claude-code"
      "cursor-cli"
    ];

  home.packages = with pkgs; [
    (python3.withPackages (ps: [
      ps.llm
      ps.llm-gemini
    ]))
    tree
    wemux
    fzf
    cursor-cli
    mongosh
    nixfmt-rfc-style
    npins
    pre-commit
    claude-code
  ];

  programs.home-manager.enable = true;
}
