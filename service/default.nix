{ lib, ... }:

{
  lib = import ./lib.nix { inherit lib; };
}
