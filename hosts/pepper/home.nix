{
  config,
  pkgs,
  lib,
  ...
}:
let
  swaylock = "${pkgs.procps}/bin/pidof swaylock || ${pkgs.swaylock}/bin/swaylock -f -i ${config.home.homeDirectory}/Documents/wallpaper.jpg -s fill";
  lock = "${pkgs.systemd}/bin/loginctl lock-session";
in
{
  services.swayidle = {
    enable = true;
    events = {
      lock = swaylock;
      before-sleep = swaylock;
    };
  };

  wayland.windowManager.sway.config.keybindings = lib.mkOptionDefault {
    "Mod4+Tab" = lib.mkForce "exec ${lock}";
    "Mod4+Shift+Tab" = lib.mkForce "exec ${lock}";
  };

  wayland.windowManager.sway.config.input = lib.mkOptionDefault {
    "type:touchpad".tap = "enabled";
  };
}
