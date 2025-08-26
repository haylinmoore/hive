{
  config,
  lib,
  pkgs,
  ...
}:

{
  services.bird = {
    enable = true;
    config = builtins.readFile ./bird.conf;
    autoReload = false;
  };

  system.activationScripts.bird-reload = lib.stringAfter [ "etc" ] ''
    if ${pkgs.systemd}/bin/systemctl is-active --quiet bird; then
      ${pkgs.bird3}/bin/birdc configure || true
    fi
  '';

  networking.firewall.allowedTCPPorts = [ 179 ]; # BGP
  networking.firewall.allowedUDPPorts = [
    3784
    3785
  ]; # BFD
}
