# Local LLM testing on astrid's Radeon 8060S (Strix Halo, Vulkan).
# Weights live outside the store in ~/models; fetch with:
#   curl -L -o ~/models/Huihui-Qwen3.5-35B-A3B-abliterated.i1-Q6_K.gguf \
#     https://huggingface.co/mradermacher/Huihui-Qwen3.5-35B-A3B-abliterated-i1-GGUF/resolve/main/Huihui-Qwen3.5-35B-A3B-abliterated.i1-Q6_K.gguf
{ pkgs, config, ... }:
let
  modelId = "qwen3.5-35b-a3b-abliterated";
  contextWindow = 131072;

  qwen-server = pkgs.writeShellApplication {
    name = "qwen-server";
    runtimeInputs = [ pkgs.llama-cpp-vulkan ];
    text = ''
      # OpenAI-compatible API at http://127.0.0.1:8080/v1
      exec llama-server \
        -m "${config.home.homeDirectory}/models/Huihui-Qwen3.5-35B-A3B-abliterated.i1-Q6_K.gguf" \
        --alias ${modelId} \
        --host 127.0.0.1 --port 8080 \
        -ngl 999 -fa on --no-mmap \
        -c ${toString contextWindow} --jinja \
        --temp 0.6 --top-p 0.95 --top-k 20 --min-p 0 \
        "$@"
    '';
  };
in
{
  home.packages = [
    pkgs.llama-cpp-vulkan
    pkgs.pi-coding-agent
    qwen-server
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
          name = "Qwen3.5 35B-A3B abliterated (local)";
          reasoning = true;
          inherit contextWindow;
          maxTokens = 32768;
        }
      ];
    };
  };
}
