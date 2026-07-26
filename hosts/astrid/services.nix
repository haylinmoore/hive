{ pkgs, ... }:
{
  # Services
  services.openssh.enable = true;
  services.printing.enable = true;
  services.lldpd.enable = true;
  services.tailscale.enable = true;

  # Remap Caps Lock to Escape
  services.interception-tools = {
    enable = true;
    plugins = [ pkgs.interception-tools-plugins.caps2esc ];
    udevmonConfig = ''
      - JOB: "${pkgs.interception-tools}/bin/intercept -g $DEVNODE | ${pkgs.interception-tools-plugins.caps2esc}/bin/caps2esc -m 1 | ${pkgs.interception-tools}/bin/uinput -d $DEVNODE"
        DEVICE:
          EVENTS:
            EV_KEY: [KEY_CAPSLOCK]
    '';
  };
}
