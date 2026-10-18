# Evaluated through `colmena eval`, which hands it every node.
{ nodes, lib, ... }:

let
  dns = import ./lib.nix { inherit lib; };

  zones = lib.mapAttrs' (
    file: _: lib.nameValuePair (lib.removeSuffix ".nix" file) (import ./zones/${file} { inherit dns; })
  ) (builtins.readDir ./zones);

  zoneOf =
    fqdn:
    let
      matches = lib.filter (zone: fqdn == zone || lib.hasSuffix ".${zone}" fqdn) (lib.attrNames zones);
    in
    if matches == [ ] then
      null
    else
      lib.last (lib.sort (a: b: lib.stringLength a < lib.stringLength b) matches);

  relative = zone: fqdn: if fqdn == zone then "@" else lib.removeSuffix ".${zone}" fqdn;
  absolute = name: if lib.hasSuffix "." name then name else "${name}.";

  declared = lib.concatLists (
    lib.mapAttrsToList (
      node: n:
      lib.mapAttrsToList (fqdn: value: {
        inherit node fqdn;
        value = removeAttrs value [ "_module" ];
      }) n.config.dns.records
    ) nodes
  );

  merged = lib.mapAttrs (
    fqdn: defs:
    let
      distinct = lib.unique (map (d: removeAttrs d.value [ "fromVhost" ]) defs);
    in
    if lib.length distinct > 1 then
      throw "dns: ${fqdn} is declared differently by ${lib.concatMapStringsSep ", " (d: d.node) defs}"
    else
      (lib.head defs).value
  ) (lib.groupBy (d: d.fqdn) declared);

  hostRecords = lib.concatLists (
    lib.mapAttrsToList (
      fqdn: r:
      let
        zone = zoneOf fqdn;
        name = relative zone fqdn;
      in
      if zone == null then
        if r.fromVhost then [ ] else throw "dns: ${fqdn} is not in a managed zone"
      else
        map (record: record // { inherit zone; }) (
          map (dns.A name) r.A
          ++ map (dns.AAAA name) r.AAAA
          ++ lib.optional (r.CNAME != null) (dns.CNAME name (absolute r.CNAME))
          ++ map (target: dns.NS name (absolute target)) r.NS
          ++ map (dns.TXT name) r.TXT
        )
    ) merged
  );

  ignores = map (
    fqdn:
    let
      zone = zoneOf fqdn;
    in
    if zone == null then
      throw "dns: ignored name ${fqdn} is not in a managed zone"
    else
      dns.IGNORE (relative zone fqdn) // { inherit zone; }
  ) (lib.unique (lib.concatMap (n: n.config.dns.ignore) (lib.attrValues nodes)));

  byZone = lib.groupBy (r: r.zone) (hostRecords ++ ignores);
in
{
  zones = lib.mapAttrs (
    name: zone:
    zone
    // {
      records = zone.records ++ map (r: removeAttrs r [ "zone" ]) (byZone.${name} or [ ]);
    }
  ) zones;
}
