# Temporary mitigations for nginx2root (https://nginx2root.com/, 2026-11).
#
# nginx < 1.30.7 / 1.31.9 (and every openresty so far) has remotely reachable
# memory corruption in HTTP/2 (CVE-2026-123007, CVE-2026-123633) and HTTP/3
# (CVE-2026-121810); the published chain then reaches root through a kernel
# SCTP use-after-free that has no fix yet.
#
# Revert the nginx half once the nixpkgs pin has nginx >= 1.30.7 and zoe's
# openresty is patched: delete the virtualHosts block below and restore the
# `Alt-Svc 'h3=":443"; ma=86400'` headers. Keep the SCTP block until the kernel
# is fixed (nothing in the hive uses SCTP).
{ lib, ... }:
{
  # Force HTTP/2, HTTP/3 and QUIC off on every vhost, including ones that
  # never set them (the NixOS module defaults http2 to true).
  options.services.nginx.virtualHosts = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        config = {
          http2 = lib.mkForce false;
          http3 = lib.mkForce false;
          http3_hq = lib.mkForce false;
          quic = lib.mkForce false;
        };
      }
    );
  };

  config = {
    # Refuse to load SCTP at all, including kernel autoload from an
    # unprivileged socket(..., IPPROTO_SCTP) call.
    boot.blacklistedKernelModules = [ "sctp" ];
    boot.extraModprobeConfig = ''
      install sctp /run/current-system/sw/bin/false
    '';
  };
}
