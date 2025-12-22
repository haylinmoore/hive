{
  lib,
  ...
}:

{
  imports = [
    ../work.nix
  ];

  home.stateVersion = lib.mkForce "25.11";
}
