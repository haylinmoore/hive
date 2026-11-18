{
  config,
  lib,
  pkgs,
  ...
}:

{
  services.nginx.virtualHosts."archive.hayl.in" = {
    forceSSL = true;
    useACMEHost = "hayl.in";
    http2 = true;
    http3 = true;
    quic = true;
    root = "/bulk/archive";
    extraConfig = ''
      add_header Alt-Svc 'clear' always; # nginx2root: h3 off, see nixos/shared/mitigations.nix
      autoindex on;
    '';
  };
}
