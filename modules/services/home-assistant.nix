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
    ];
    customComponents = [
      pkgs.home-assistant-custom-components.luxer_one
      hive.python.ha.meshcore
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

  proxySites.ha = {
    domain = "ha.uwu.estate";
    proxyUri = "http://[::1]:8123/";
    useACMEHost = "uwu.estate";
  };
}
