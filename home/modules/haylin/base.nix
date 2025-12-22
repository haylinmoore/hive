{
  pkgs,
  lib,
  hive,
  ...
}:
{
  imports = [
    ../nvim-ng/home.nix
    ../git.nix
  ];

  programs.git.userEmail = lib.mkDefault "me@haylinmoore.com";

  home.packages = with pkgs; [
    # utils
    tmux
    npins
    curl
    dig
    q
    mtr
    tree
    git
    sapling
    gh
    zsh
    pulsemixer
    ffmpeg
    htop
    killall
    zip
    unzip
    nmap
    numbat
    ripgrep
    file
    whois
    bluetui
    mpv
    age
    fzf
    nix-init
    nix-diff
    nix-update
    dust
    wireshark

    # nix tools
    nil
    claude-code
  ];

  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    matchBlocks = (import ./hosts.nix).hosts;
  };

  programs.zsh = {
    enable = true;
    oh-my-zsh = {
      enable = true;
      plugins = [ "git" ];
      theme = "lambda";
    };
    shellAliases = {
      cg = "sudo nix-collect-garbage --delete-older-than 7d";
      nixpkgs_sync = "gh repo sync haylinmoore/nixpkgs -b master";
    };
  };

  programs.zoxide = {
    enable = true;
  };

  programs.zellij = {
    enable = true;
    enableZshIntegration = false;
  };

  xdg.configFile."zellij/config.kdl".text = ''
    copy_command "wl-copy"
    simplified_ui  true
    pane_frames false

    copy_on_select true
    on_force_close "quit"
    session_serialization false
    pane_viewport_serialization false
    disable_session_metadata false

    ui {
        pane_frames {
            hide_session_name true
        }
    }
  '';

  programs.helix = {
    enable = true;
    languages.language-server.texlab.config.texlab = {
      build.onSave = true;
      build.forwardSearchAfter = true;
      forwardSearch.executable = "zathura";
      forwardSearch.args = [
        "--synctex-forward"
        "%l:1:%f"
        "%p"
      ];
      chktex.onEdit = true;
    };
  };
}
