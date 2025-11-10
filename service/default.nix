{ lib, ... }:

{
  lib = import ./lib.nix { inherit lib; };
  wrapVirtualHost = import ./wrapVirtualHost.nix { inherit lib; };
}
