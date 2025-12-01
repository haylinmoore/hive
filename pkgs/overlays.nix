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

  (
    final: prev:
    infuse prev {
      # claude-code updates enough I want to bump it myself
      claude-code.__output.version.__assign = "2.0.55";
      claude-code.__output.src.__output.urls.__assign = [
        "https://registry.npmjs.org/@anthropic-ai/claude-code/-/claude-code-2.0.55.tgz"
      ];
      claude-code.__output.src.__output.outputHash.__assign =
        "sha256-wsjOkNxuBLMYprjaZQyUZHiqWl8UG7cZ1njkyKZpRYg=";
    }
  )
]
