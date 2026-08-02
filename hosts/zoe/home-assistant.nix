{
  config,
  lib,
  pkgs,
  hive,
  ...
}:

{
  # Add the home-assistant user to dialout group for serial port access
  users.users.hass.extraGroups = [ "dialout" ];

  services.home-assistant = {
    enable = true;
    extraComponents = [
      "esphome"
      "met"
      "group"
      "radio_browser"
      "google_translate"
      "homeassistant_hardware"
      "zha"
      "homekit"
      "aranet"
      "matter"
      "otbr"
      "thread"
    ];
    customComponents = [
      pkgs.home-assistant-custom-components.luxer_one
      hive.pkgs.python.ha.meshcore
    ];
    extraPackages =
      python3Packages: with python3Packages; [
        # Mentioned in crashes
        aiohomekit
        python-otbr-api
        pyatv
      ];
    config = {
      default_config = { };
      http = {
        #server_host = "::1";
        trusted_proxies = [
          "::1"
          "127.0.0.1"
        ];
        use_x_forwarded_for = true;
      };
      "scripts ui" = "!include scripts.yaml";
      "automation ui" = "!include automations.yaml";
      "automation manual" =
        let
          upstairsButtons = [
            "c1fb487579953c8e36567d604e906381"
            "8f9f8c0d2ecabaf1a59632e6f38970dc"
          ];

          makeButtonTriggers =
            triggerType:
            map (device_id: {
              inherit device_id;
              domain = "zha";
              type = triggerType;
              subtype = "button";
              trigger = "device";
            }) upstairsButtons;
        in
        [
          {
            alias = "Upstairs Toggle Lights";
            triggers = makeButtonTriggers "remote_button_short_press";
            actions = [
              {
                action = "light.toggle";
                target = {
                  entity_id = "light.upstairs_lights";
                };
              }
            ];
            mode = "single";
          }
          {
            alias = "Upstairs Dim Lights";
            triggers = makeButtonTriggers "remote_button_long_press";
            actions = [
              {
                action = "light.turn_off";
                data = {
                  transition = 1;
                };
                target = {
                  entity_id = "light.room_lamp";
                };
              }
              {
                action = "light.turn_on";
                data = {
                  brightness_pct = 25;
                  transition = 1;
                };
                target = {
                  entity_id = "light.stair_lights";
                };
              }
            ];
            mode = "single";
          }
          {
            alias = "Upstairs Normalize Lights";
            description = "";
            triggers = makeButtonTriggers "remote_button_double_press";
            actions = [
              {
                action = "light.turn_on";
                data = {
                  brightness_pct = 100;
                  color_temp_kelvin = 3800;
                };
                target = {
                  area_id = "bedroom";
                };
              }
            ];
            mode = "single";
          }
        ];
    };
  };
  systemd.tmpfiles.rules = [
    "f ${config.services.home-assistant.configDir}/automations.yaml 0755 hass hass"
  ];

  # Local Matter controller for HA's "Matter" integration. Not exposed
  # publicly - HA talks to it over localhost, and commissioning happens
  # over the LAN only (zoe's firewall is disabled network-wide already).
  services.matter-server = {
    enable = true;
    # Auto-detection picks "None" under the service's network sandboxing,
    # which breaks mDNS advertisement ("Network is unreachable").
    extraArgs.primary-interface = "enp4s0";
  };

  # Work around a python-matter-server startup crash: it fetches PAA root
  # certs from the DCL on start, and one malformed cert there makes the strict
  # rust-based x509 parser raise an uncaught ValueError, killing start() so the
  # server never binds 5580. The fetch is skipped when <cache>/certs/.version
  # is under 24h old, so keep it fresh to pin the already-cached cert set.
  # Remove once upstream tolerates the bad cert (home-assistant/python-matter-server).
  systemd.services.matter-server.preStart = ''
    mkdir -p "$CACHE_DIRECTORY/certs"
    touch "$CACHE_DIRECTORY/certs/.version"
  '';

  services.openthread-border-router = {
    enable = true;
    backboneInterfaces = [ "enp4s0" ];
    radio = {
      device = "/dev/serial/by-id/usb-Itead_Sonoff_Zigbee_3.0_USB_Dongle_Plus_V2_8a3aa8949378f011b4d8ace70ba521c7-if00-port0";
      baudRate = 460800;
      flowControl = false;
    };
    rest = {
      listenAddress = "::";
      listenPort = 8081;
    };
    web = {
      enable = true;
      listenAddress = "::";
      listenPort = 58082;
    };
  };

  proxySites.ha = {
    domain = "ha.uwu.estate";
    proxyUri = "http://[::1]:8123/";
    useACMEHost = "uwu.estate";
  };
}
