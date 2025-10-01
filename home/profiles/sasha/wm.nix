{
  pkgs,
  lib,
  hive,
  ...
}:
{

  home.pointerCursor = {
    name = "Adwaita";
    package = pkgs.adwaita-icon-theme;
    size = 24;
    x11 = {
      enable = true;
      defaultCursor = "Adwaita";
    };
  };
  dconf.settings = {
    "org/gnome/desktop/interface" = {
      color-scheme = "prefer-dark";
    };
  };

  programs.rofi = {
    enable = true;
    package = pkgs.rofi;
  };

  wayland.windowManager.sway = {
    enable = true;
    config = rec {
      defaultWorkspace = "workspace number 1";
      terminal = "kitty";
      modifier = "Mod4";
      window.titlebar = false;
      menu = "rofi -show run";
      bars = [
        {
          command = "waybar";
          fonts = {
            names = [ "Berkeley Monospace" ];
            size = 11.0;
          };
        }
      ];
      startup = [ { command = "swaybg -m fill -i ~/Documents/wallpaper.jpg"; } ];
      keybindings = lib.mkOptionDefault {
        "Mod4+Shift+D" = "exec ${pkgs.rofi}/bin/rofi -show ssh";
        "Mod4+M" = "exec ${hive.pkgs.patch.kitty}/bin/kitty -T qalc ${pkgs.libqalculate}/bin/qalc";
        "Mod4+Shift+Slash" = "kill";
        "XF86AudioRaiseVolume" = "exec ${pkgs.pulsemixer}/bin/pulsemixer --change-volume +5";
        "XF86AudioLowerVolume" = "exec ${pkgs.pulsemixer}/bin/pulsemixer --change-volume -5";
        "XF86AudioMute" = "exec ${pkgs.pulsemixer}/bin/pulsemixer --toggle-mute";
        "XF86MonBrightnessDown" = "exec ${pkgs.brightnessctl}/bin/brightnessctl s 25-";
        "XF86MonBrightnessUp" = "exec ${pkgs.brightnessctl}/bin/brightnessctl s +25";
        "Print" = "exec ${pkgs.grim}/bin/grim - | wl-copy";
        "Mod4+Print" = ''exec ${pkgs.grim}/bin/grim -g "$(${pkgs.slurp}/bin/slurp -d)" - | wl-copy'';
        "Mod4+Tab" = "exec ${pkgs.swaylock}/bin/swaylock -i ~/Documents/wallpaper.jpg -s fill";
        "Mod4+Shift+Tab" =
          "exec ${pkgs.bash}/bin/bash -c '${pkgs.grim}/bin/grim /tmp/screenshot.png && ${pkgs.swaylock}/bin/swaylock -i /tmp/screenshot.png -s fill'";
      };
      input = lib.mkOptionDefault {
        "10248:536:ASUF1208:00_2808:0218_Touchpad" = {
          "click_method" = "clickfinger";
          "tap" = "enabled";
          "dwt" = "disabled";
        };
      };
    };
    extraConfig = ''
      for_window [app_id="firefox" title="^Picture-in-Picture$"] floating enable, resize set 360 200, move position 1077 667, sticky enable, border none
      for_window [app_id="kitty" title="qalc"] floating enable, resize set 710 246, move position 354 196, sticky enable, border none
      input * xkb_options compose:ralt
      output * adaptive_sync on
    '';
  };

  programs.waybar = {
    enable = true;

    settings = {
      mainBar = {
        layer = "bottom";
        position = "top";
        height = 32;

        modules-left = [
          "custom/logo"
          "sway/workspaces"
          "sway/mode"
        ];
        modules-center = [ "clock" ];
        modules-right = [
          "pulseaudio"
          "battery"
        ];

        "custom/logo" = {
          format = "λ";
          tooltip = false;
        };

        "sway/workspaces" = {
          disable-scroll = true;
          all-outputs = true;
          disable-click = false;
        };

        "sway/mode" = {
          tooltip = false;
        };

        "clock" = {
          interval = 60;
          format = "{:%b %d • %H:%M}";
        };

        "battery" = {
          tooltip = true;
          "format" = "{icon}  {capacity}%";
          "format-icons" = [
            ""
            ""
            ""
            ""
            ""
          ];
        };

        "pulseaudio" = {
          "format" = "{icon}  {volume}%";
          "format-icons" = {
            "headphone" = "";
            "phone" = "";
            "phone-muted" = "";
            "portable" = "";
            "car" = "";
            "default-muted" = "";
            "default" = [
              ""
              ""
              ""
            ];
          };
        };
      };
    };

    style = ''
      * {
        border: none;
        border-radius: 0;
        padding: 0;
        margin: 0;
        font-size: 12px;
        font-family: "Berkeley Mono Regular", monospace;
      }

      window#waybar {
        background: rgba(0,0,0,0);
        color: #ffffff;
      }

      #custom-logo {
        font-size: 18px;
        margin: 0;
        margin-left: 7px;
        margin-right: 8px;
        padding: 0;
      }

      #workspaces button {
        color: #ffffff;
      }
      #workspaces button:hover, #workspaces button:active {
        background-color: #292828;
        color: #ffffff;
      }
      #workspaces button.focused {
        background-color: #383737;
      }

      #battery {
        margin-left: 7px;
        margin-right: 12px;
      }
    '';
  };
}
