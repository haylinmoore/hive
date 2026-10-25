{ dns }:
with dns;
let
  hosts = import ../../nixos/mesh/hosts.nix;
  address = (import ../../tools/yggdrasil.nix { inherit lib; }).address;
  hubs = lib.filterAttrs (_: h: h ? endpoint) hosts;
in
{
  registrar = "porkbun";
  provider = "bunny";
  ttl = 60;
  records =
    lib.mapAttrsToList (name: h: AAAA "${name}.hive" (address h.publicKey)) hosts
    ++ lib.mapAttrsToList (_: h: AAAA "dash.hive" (address h.publicKey)) hubs;
}
