{
  config,
  lib,
  pkgs,
  modulesPath,
  hive,
  ...
}:
{
  # Audio configuration
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;

  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = true;
    wireplumber.enable = true;

    extraConfig.pipewire."90-speaker-eq"."context.modules" = [
      {
        name = "libpipewire-module-filter-chain";
        args = {
          "node.description" = "Speaker EQ";
          "media.name" = "Speaker EQ";
          "audio.channels" = 2;
          "audio.position" = [
            "FL"
            "FR"
          ];
          "filter.graph".inputs = [
            "eq:In 1"
            "eq:In 2"
          ];
          "filter.graph".outputs = [
            "eq:Out 1"
            "eq:Out 2"
          ];
          "filter.graph".nodes = [
            {
              type = "builtin";
              name = "eq";
              label = "param_eq";
              config.filters = [
                {
                  type = "bq_highshelf";
                  freq = 0;
                  Q = 1.0;
                  gain = -6.0;
                }
                {
                  type = "bq_highpass";
                  freq = 100;
                  Q = 0.707;
                }
                {
                  type = "bq_peaking";
                  freq = 170;
                  Q = 0.9;
                  gain = 4.0;
                }
                {
                  type = "bq_peaking";
                  freq = 3200;
                  Q = 1.4;
                  gain = -3.5;
                }
                {
                  type = "bq_peaking";
                  freq = 6500;
                  Q = 2.0;
                  gain = -2.0;
                }
                {
                  type = "bq_highshelf";
                  freq = 11000;
                  Q = 0.7;
                  gain = 2.0;
                }
              ];
            }
          ];
          "capture.props" = {
            "node.name" = "effect_input.speaker_eq";
            "media.class" = "Audio/Sink";
            "filter.smart" = true;
            "filter.smart.name" = "speaker-eq";
            "filter.smart.target"."node.name" = "alsa_output.pci-0000_00_1f.3-platform-sof_sdw.HiFi__Speaker__sink";
          };
          "playback.props" = {
            "node.name" = "effect_output.speaker_eq";
            "node.passive" = true;
          };
        };
      }
    ];
  };

  hardware.firmware = [
    pkgs.sof-firmware
    pkgs.alsa-firmware
  ];

  environment.systemPackages = with pkgs; [
    pavucontrol
    alsa-ucm-conf
  ];

  environment = {
    sessionVariables.ALSA_CONFIG_UCM2 = "${pkgs.alsa-ucm-conf}/share/alsa/ucm2";
  };
}
