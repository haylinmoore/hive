# Local LLM testing on astrid's Radeon 8060S (Strix Halo, Vulkan).
#
# `llm-server` runs llama-server in router mode: one endpoint at
# http://127.0.0.1:8080/v1 that loads whichever model a request names (one at a
# time), so switching models in pi with /model just works.
#
# Weights live outside the store in ~/models; fetch each model's `url` into it.
# llama-cpp-vulkan is pinned to a master commit in pkgs/overlays.nix for Inkling 2 support.
{
  pkgs,
  lib,
  config,
  ...
}:
let
  modelsDir = "${config.home.homeDirectory}/models";
  contextWindow = 131072;

  models = {
    inkling-2-mini = {
      name = "Inkling 2 Mini (local)";
      file = "Inkling-2-Mini-UD-Q8_K_XL.gguf";
      url = "https://huggingface.co/unsloth/Inkling-2-Mini-GGUF/resolve/main/Inkling-2-Mini-UD-Q8_K_XL.gguf";
      # generation_config.json: temperature 1.0, no other filters
      args = {
        temp = 1.0;
        top-p = 1.0;
        top-k = 0;
        min-p = 0;
      };
    };
    inkling-2-mini-unbound = {
      name = "Inkling 2 Mini Unbound v2.1 (local, abliterated)";
      file = "Inkling-2-Mini-Unbound-v2.1-Q8_0.gguf";
      url = "https://huggingface.co/nokeepai/Inkling-2-Mini-Unbound-GGUF/resolve/main/Inkling-2-Mini-Unbound-v2.1-Q8_0.gguf";
      # Model card: thinking on, temp 0.6, top-p 0.95, cap thinking, no DRY
      args = {
        temp = 0.6;
        top-p = 0.95;
        top-k = 0;
        min-p = 0;
      };
    };
  };

  presets = pkgs.writeText "llm-models.ini" (
    "version = 1\n\n"
    + lib.generators.toINI { } (
      {
        "*" = {
          c = contextWindow;
          # Both model cards warn about runaway reasoning loops.
          reasoning-budget = 8192;
        };
      }
      // lib.mapAttrs (_: m: { model = "${modelsDir}/${m.file}"; } // m.args) models
    )
  );

  llm-server = pkgs.writeShellApplication {
    name = "llm-server";
    runtimeInputs = [ pkgs.llama-cpp-vulkan ];
    text = ''
      exec llama-server \
        --models-preset ${presets} \
        --models-max 1 \
        --host 127.0.0.1 --port 8080 \
        "$@"
    '';
  };
in
{
  home.packages = [
    pkgs.llama-cpp-vulkan
    pkgs.pi-coding-agent
    llm-server
  ];

  home.file.".pi/agent/models.json".text = builtins.toJSON {
    providers.local = {
      baseUrl = "http://127.0.0.1:8080/v1";
      api = "openai-completions";
      apiKey = "local";
      compat = {
        supportsDeveloperRole = false;
        supportsReasoningEffort = false;
      };
      models = lib.mapAttrsToList (id: m: {
        inherit id;
        inherit (m) name;
        reasoning = true;
        inherit contextWindow;
        maxTokens = 32768;
      }) models;
    };
  };
}
