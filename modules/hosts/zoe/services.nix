{
  config,
  lib,
  pkgs,
  ...
}:

{

  services.duckdns = {
    enable = true;
    tokenFile = "/run/secrets/duckdns";
    domains = [
      "uwu-estate"
    ];
  };

  services.home-assistant = {
    enable = true;
    extraComponents = [
      "esphome"
      "met"
      "radio_browser"
      "google_translate"
      "homeassistant_hardware"
      "zha"
      "homekit"
    ];
    extraPackages = python3Packages: with python3Packages; [ ];
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
    };
  };
  systemd.tmpfiles.rules = [
    "f ${config.services.home-assistant.configDir}/automations.yaml 0755 hass hass"
  ];

  proxySites = {
    ha = {
      domain = "ha.uwu.estate";
      proxyUri = "http://[::1]:8123/";
    };
  };
}
