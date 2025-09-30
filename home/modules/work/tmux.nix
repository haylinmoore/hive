{
  config,
  lib,
  pkgs,
  ...
}:

{
  programs.tmux = {
    enable = true;
    terminal = "xterm-256color";
    extraConfig = ''
      set-option -ga terminal-overrides ",xterm-256color:Tc"
      set-option -ga terminal-overrides ",*256col*:Tc"
      set-option -ga terminal-overrides ",xterm-kitty:Tc"
      set -g default-terminal "tmux-256color"

      set -s set-clipboard on
      set -sg escape-time 0
    '';
  };
}
