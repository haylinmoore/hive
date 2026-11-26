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
      #claude-code.__output.version.__assign = "2.0.55";
      #claude-code.__output.src.__output.urls.__assign = [
      #  "https://registry.npmjs.org/@anthropic-ai/claude-code/-/claude-code-2.0.55.tgz"
      #];
      #claude-code.__output.src.__output.outputHash.__assign =
      #  "sha256-wsjOkNxuBLMYprjaZQyUZHiqWl8UG7cZ1njkyKZpRYg=";
    }
  )

  # llama.cpp master for Inkling 2 support (merged after the v1.0.0 release).
  # Drop this once nixpkgs ships a release containing ggml-org/llama.cpp#32217 and #32381.
  # Master moved the web UI to a multi-package ui/ tree that defaults off
  # (LLAMA_BUILD_UI=OFF), so the npm build steps from nixpkgs are stripped; the
  # OpenAI-compatible API is unaffected.
  (final: prev: {
    llama-cpp-vulkan = prev.llama-cpp-vulkan.overrideAttrs (old: {
      version = "1.0.0-unstable-2026-11-26";
      src = final.fetchFromGitHub {
        owner = "ggml-org";
        repo = "llama.cpp";
        rev = "a8ad690ca9239ea35db4de47e00cf632d01895fd";
        hash = "sha256-pbSWqm+a3rnm2TB1TGkGZhzNEtTSORLI/kwZPAaTSJ8=";
      };
      npmDeps = null;
      preConfigure = "";
      nativeBuildInputs = builtins.filter (
        p: p != final.npmHooks.npmConfigHook && p != final.nodejs_latest
      ) old.nativeBuildInputs;
      cmakeFlags =
        builtins.filter (f: !(final.lib.hasPrefix "-DLLAMA_BUILD_COMMIT" f)) old.cmakeFlags
        ++ [ (final.lib.cmakeFeature "LLAMA_BUILD_COMMIT" "a8ad690") ];
    });
  })

  # Regular overlay for slack wayland wrapper (avoids rebuild)
  (final: prev: {
    slack = final.symlinkJoin {
      name = "slack-wayland";
      paths = [ prev.slack ];
      buildInputs = [ final.makeWrapper ];
      postBuild = ''
        wrapProgram $out/bin/slack \
          --add-flags "--ozone-platform=wayland"
      '';
    };
  })
]
