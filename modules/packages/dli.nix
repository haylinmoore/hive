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
    rev = "68224b3381727625b01f2d3523122014b7cc355d";
    sha256 = "sha256-fduZMLuANnRj5gtYXL/g6ryRXaBAzGj3N6ABpM7FlhM=";
  };

  vendorHash = "sha256-7bJXyMUJNRNo1jDPDDEgj0uh647eyrF14Qe8DWfHyLs=";

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
