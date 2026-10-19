{ pkgs, ... }:

pkgs.writers.writePython3Bin "arin-sync" {
  libraries = [ pkgs.python3Packages.dnspython ];
  flakeIgnore = [ "E501" ];
} (builtins.readFile ./sync.py)
