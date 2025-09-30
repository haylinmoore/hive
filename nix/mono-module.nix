{ pkgs, lib, ... }:
{
  _module.args.mono = import ../default.nix;
}
