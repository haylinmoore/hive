{ lib }:

let
  nameservers = [
    "ns1.aconite.systems."
    "ns2.aconite.systems."
  ];

  line =
    r:
    let
      a = r.args;
      at = lib.elemAt a;
    in
    if
      lib.elem r.type [
        "A"
        "AAAA"
        "CNAME"
        "NS"
        "PTR"
      ]
    then
      "${at 0} IN ${r.type} ${at 1}"
    else if r.type == "TXT" then
      "${at 0} IN TXT \"${lib.escape [ "\"" "\\" ] (at 1)}\""
    else if r.type == "MX" then
      "${at 0} IN MX ${toString (at 1)} ${at 2}"
    else if r.type == "SRV" then
      "${at 0} IN SRV ${lib.concatMapStringsSep " " toString (lib.drop 1 a)}"
    else
      throw "dns: ${r.type} records can't be served by knot";
in
zone:
lib.concatStringsSep "\n" (
  [
    "$TTL ${toString zone.ttl}"
    "@ IN SOA ${lib.head nameservers} hostmaster.aconite.systems. 1 3600 900 1209600 3600"
  ]
  ++ map (ns: "@ IN NS ${ns}") nameservers
  ++ map line zone.records
)
+ "\n"
