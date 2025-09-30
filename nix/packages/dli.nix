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
    rev = "609a58f5f149188ce2c1c0f28beec88494c8722a";
    hash = "sha256-NDGYL7ZPIZIipqLYy0r9V9vSAtvVgcDpGai0UvELYyo=";
  };

  vendorHash = "sha256-FFUjjLyLSBTrGTyEKMfd3/j8P32zMt5HrO29g72MtK4=";

  meta = {
    description = "Dynamic DNS CLI tool with support for multiple providers";
    homepage = "https://github.com/haylinmoore/dli";
    license = lib.licenses.mit;
    maintainers = [ ];
    mainProgram = "dli";
  };
}
