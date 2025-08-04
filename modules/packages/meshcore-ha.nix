{
  lib,
  fetchFromGitHub,
  buildHomeAssistantComponent,
  pkgs,
}:

let
  pycayennelpp = pkgs.callPackage ./pycayennelpp.nix { };
  meshcore = pkgs.callPackage ./meshcore.nix { inherit pycayennelpp; };
in
buildHomeAssistantComponent rec {
  owner = "awolden";
  domain = "meshcore";
  version = "2.0.9";

  src = fetchFromGitHub {
    owner = "awolden";
    repo = "meshcore-ha";
    rev = "657b83ea0383252666dc4cea3243b44a1c2eb669";
    hash = "sha256-3ep7FkucjX3IrAHogxfY9sWtHCqrevwyc2GpqV1BMvI=";
  };

  propagatedBuildInputs = [
    meshcore
    pycayennelpp
    pkgs.python3Packages.bleak
    pkgs.python3Packages.pyserial-asyncio
  ];

  # Skip manifest check since meshcore-cli doesn't exist on PyPI
  dontCheckManifest = true;

  meta = {
    description = "Home Assistant integration for monitoring and controlling MeshCore radio networks";
    homepage = "https://github.com/awolden/meshcore-ha";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ haylinmoore ];
  };
}
