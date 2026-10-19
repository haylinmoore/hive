{ pkgs, lib, ... }:

let
  org = "HWSL-35";
  poc = "HAYLI-ARIN";

  arin = import ./lib.nix;
  blocks = import ./blocks.nix { inherit arin; };
  sets = import ./sets.nix;

  zones = map (lib.removeSuffix ".nix") (lib.attrNames (builtins.readDir ../dns/zones));

  normalize =
    block: b:
    let
      origin = b.origin or (throw "arin: ${block} needs an origin");
      entry = e: if builtins.isString e then { prefix = e; } else e;
      withOrigin = e: { inherit origin; } // entry e;
      rdns = lib.mapAttrs (
        zone: d: if lib.elem zone zones then d else throw "arin: ${zone} has no zone file in dns/zones"
      ) (b.rdns or { });
    in
    {
      route = map withOrigin (b.route or [ ]);
      route6 = map withOrigin (b.route6 or [ ]);
      rpki = map withOrigin (b.rpki or [ ]);
      inherit rdns;
    };
in
{
  state = pkgs.writeText "arin.json" (
    builtins.toJSON {
      inherit org poc sets;
      blocks = lib.mapAttrs normalize blocks;
    }
  );
}
