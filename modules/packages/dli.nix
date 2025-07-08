{
  lib,
  buildGoModule,
  fetchFromGitHub,
}:

buildGoModule rec {
  pname = "dli";
  version = "0.0.0-unstable-2025-01-08";

  src = fetchFromGitHub {
    owner = "haylinmoore";
    repo = "dli";
    rev = "ae42c028f82d432b68ae27b42b6f47af94d1a008";
    sha256 = "sha256-FTNKWpyK5saAzVEMCFq5FeTKWyVKf2MoGxVp2clg0cQ=";
  };

  vendorHash = "sha256-cuwJ3MlNFnfzgA929Csjq7hkdWyLKCHoc+VeZCkQSDQ=";

  ldflags = [
    "-s"
    "-w"
  ];

  doCheck = false; # DNS-related tests may require network access

  meta = {
    description = "Dynamic DNS CLI tool with support for multiple providers";
    homepage = "https://github.com/haylinmoore/dli";
    license = lib.licenses.mit; # Adjust if different
    maintainers = [ ];
    mainProgram = "dli";
  };
}
