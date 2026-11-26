# Local LLM testing on astrid's Radeon 8060S (Strix Halo, Vulkan).
# Weights live outside the store in ~/models; fetch with:
#   curl -L -o ~/models/Inkling-2-Mini-UD-Q8_K_XL.gguf \
#     https://huggingface.co/unsloth/Inkling-2-Mini-GGUF/resolve/main/Inkling-2-Mini-UD-Q8_K_XL.gguf
# llama-cpp-vulkan is pinned to a master commit in pkgs/overlays.nix for Inkling 2 support.
{ pkgs, config, ... }:
let
  modelId = "inkling-2-mini";
  modelFile = "Inkling-2-Mini-UD-Q8_K_XL.gguf";
  contextWindow = 131072;

  llm-server = pkgs.writeShellApplication {
    name = "llm-server";
    runtimeInputs = [ pkgs.llama-cpp-vulkan ];
    text = ''
      # OpenAI-compatible API at http://127.0.0.1:8080/v1
      # Sampling per the model's generation_config.json (temperature 1.0, no other filters).
      exec llama-server \
        -m "${config.home.homeDirectory}/models/${modelFile}" \
        --alias ${modelId} \
        --host 127.0.0.1 --port 8080 \
        -c ${toString contextWindow} \
        --temp 1.0 --top-p 1.0 --top-k 0 --min-p 0 \
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
      models = [
        {
          id = modelId;
          name = "Inkling 2 Mini (local)";
          reasoning = true;
          inherit contextWindow;
          # The model card warns about long reasoning loops; keep output bounded.
          maxTokens = 32768;
        }
      ];
    };
  };
}
