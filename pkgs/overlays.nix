{
  lib,
}:
let
  infuse = (import ../tools/infuse.nix { inherit lib; }).v1.infuse;
in
[
  (
    final: prev:
    infuse prev {

      # Initial infusion to make sure the system works
      hello.__output.version.__assign = "2.12.1";
      hello.__output.src.__output.outputHash.__assign =
        "sha256-jZkUKv2SV28wsM18tCqNxoCZmLxdYH2Idh9RLibH2yA=";
    }
  )
]
