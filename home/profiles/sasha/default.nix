{
  pkgs,
  lib,
  hive,
  ...
}:
{
  imports = [
    (hive.sources.catppuccin + "/modules/home-manager")
    ./wm.nix
    ../../modules/nvim.nix
    ../../modules/git.nix
  ];

  home.stateVersion = "24.11";
  home.username = "haylin";
  home.homeDirectory = "/home/haylin";
  programs.git.userEmail = "me@haylinmoore.com";

  catppuccin = {
    enable = true;
    flavor = "mocha";
  };

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
    virtualgl
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

    # desktop apps
    firefox
    discord
    spotify
    slack
    obsidian
    signal-desktop
    gimp
    thunderbird
    zathura
    tor-browser
    libreoffice
    russ
    # cinny-desktop
    # fluffychat
    nicotine-plus
    g4music
    finamp

    # memes
    hyfetch

    #code
    nil
    kubectl
    gnumake
    ccls
    gdb
    unzip
    ncdu
    hive.tools.claude

    # games
    prismlauncher
    # balatro

    # golang
    go
    gopls
    gotools
    go-tools
    delve
  ];

  programs.kitty = {
    enable = true;
    package = hive.tools.kitty;
    settings = {
      font_family = "Berkeley Mono";
      bold_font = "Berkeley Mono Bold";
      bold_italic_font = "Berkeley Mono Bold Italic";
      italic_font = "Berkeley Mono Italic";
    };
    extraConfig = ''
      confirm_os_window_close 0
       background_opacity 0.9
    '';
  };

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
