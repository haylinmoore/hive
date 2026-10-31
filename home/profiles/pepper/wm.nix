{
  config,
  pkgs,
  lib,
  hive,
  ...
}:
let
  swaylock = "${pkgs.procps}/bin/pidof swaylock || ${pkgs.swaylock}/bin/swaylock -f -i ${config.home.homeDirectory}/Documents/wallpaper.jpg -s fill";
  lock = "${pkgs.systemd}/bin/loginctl lock-session";
  powerProfileToggle = pkgs.writeShellScript "power-profile-toggle" ''
    ppc=${pkgs.power-profiles-daemon}/bin/powerprofilesctl
    if [ "$($ppc get)" = power-saver ]; then
      $ppc set balanced
    else
      $ppc set power-saver
    fi
  '';
  brightness = pkgs.writeShellScript "brightness" ''
    PATH=${
      lib.makeBinPath [
        pkgs.brightnessctl
        pkgs.util-linux
        pkgs.coreutils
      ]
    }
    dev=/sys/class/backlight/intel_backlight
    max=$(<$dev/max_brightness)
    state=$XDG_RUNTIME_DIR/brightness-target

    exec 9>"$state.lock"
    flock 9
    t=$(cat "$state" 2>/dev/null || cat $dev/brightness)
    case $1 in
      up) t=$((t * 5 / 4 + 1)) ;;
      down) t=$((t * 4 / 5)) ;;
    esac
    t=$((t < max / 100 ? max / 100 : t > max ? max : t))
    echo $t >"$state"
    flock -u 9

    exec 8>"$state.anim"
    flock -n 8 || exit 0
    cur=$(<$dev/brightness)
    while :; do
      flock 9
      t=$(<"$state")
      if [ "$t" -eq "$cur" ]; then
        rm "$state"
        exec 8>&-
        exit 0
      fi
      flock -u 9
      d=$(((t - cur) * 2 / 5))
      [ $d -eq 0 ] && d=$((t > cur ? 1 : -1))
      cur=$((cur + d))
      brightnessctl -q s $cur
      sleep 0.01
    done
  '';
in
{
  services.swayidle = {
    enable = true;
    events = {
      lock = swaylock;
      before-sleep = swaylock;
      unlock = "${pkgs.procps}/bin/pkill -USR1 swaylock";
    };
  };

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
        "Mod4+M" = "exec ${pkgs.kitty}/bin/kitty -T qalc ${pkgs.libqalculate}/bin/qalc";
        "Mod4+Shift+Slash" = "kill";
        "XF86AudioRaiseVolume" = "exec ${pkgs.pulsemixer}/bin/pulsemixer --change-volume +5";
        "XF86AudioLowerVolume" = "exec ${pkgs.pulsemixer}/bin/pulsemixer --change-volume -5";
        "XF86AudioMute" = "exec ${pkgs.pulsemixer}/bin/pulsemixer --toggle-mute";
        "XF86AudioMedia" = "exec ${powerProfileToggle}";
        "XF86MonBrightnessDown" = "exec ${brightness} down";
        "XF86MonBrightnessUp" = "exec ${brightness} up";
        "Print" = "exec ${pkgs.grim}/bin/grim - | wl-copy";
        "Mod4+Print" = ''exec ${pkgs.grim}/bin/grim -g "$(${pkgs.slurp}/bin/slurp -d)" - | wl-copy'';
        "Mod4+Tab" = "exec ${lock}";
        "Mod4+Shift+Tab" = "exec ${lock}";
      };
      input = lib.mkOptionDefault {
        "type:touchpad".tap = "enabled";
        "type:touchpad".click_method = "clickfinger";
        "type:touch".map_to_output = "eDP-1";
      };
    };
    extraConfig = ''
      for_window [app_id="firefox" title="^Picture-in-Picture$"] floating enable, resize set 360 200, move position 1077 667, sticky enable, border none
      for_window [app_id="kitty" title="qalc"] floating enable, resize set 710 246, move position 354 196, sticky enable, border none
      input * xkb_options compose:ralt
      output * adaptive_sync on
      output eDP-1 adaptive_sync off
      output "ASUSTek COMPUTER INC ASUS PA279 0x0000C792" scale 2
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
          "power-profiles-daemon"
          "pulseaudio"
          "battery"
        ];

        "power-profiles-daemon" = {
          format = "{icon}";
          tooltip-format = "{profile}";
          format-icons = {
            default = "";
            performance = "";
            balanced = "";
            power-saver = "";
          };
        };

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

      #power-profiles-daemon {
        margin-right: 12px;
        color: #777777;
      }
      #power-profiles-daemon.power-saver {
        color: #ffffff;
      }

      #battery {
        margin-left: 7px;
        margin-right: 12px;
      }
    '';
  };
}
