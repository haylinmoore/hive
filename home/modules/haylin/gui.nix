{
  pkgs,
  lib,
  hive,
  config,
  ...
}:
{
  imports = [
    (hive.sources.catppuccin + "/modules/home-manager")
  ];

  catppuccin = {
    enable = true;
    flavor = "mocha";
  };

  home.packages = with pkgs; [
    # desktop apps
    firefox
    slack
    obsidian
    thunderbird
    zathura
    libreoffice
  ];

  programs.kitty = {
    enable = true;
    extraConfig = ''
      confirm_os_window_close 0
      background_opacity 0.9

      # Fix Ctrl-/ to send the proper terminal code
      map ctrl+slash send_text all \x1f
    '';
  };
}
