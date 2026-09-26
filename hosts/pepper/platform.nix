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

    extraLv2Packages = [
      pkgs.lsp-plugins
      pkgs.calf
    ];

    configPackages = [
      (pkgs.runCommand "dolby-speaker-tuning" { } ''
        mkdir -p $out/share/pipewire/pipewire.conf.d
        substitute ${./speaker/Dolby_Balanced.conf} \
          $out/share/pipewire/pipewire.conf.d/90-dolby-speaker.conf \
          --replace-fail @irs@ ${./speaker/Dolby_Balanced.irs}
      '')
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
