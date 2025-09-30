{ pkgs, lib }:
let
  entries = builtins.readDir ./.;
  packageNames = builtins.filter (
    name: entries.${name} == "directory" && builtins.pathExists (./${name}/default.nix)
  ) (builtins.attrNames entries);

  autoPackages = lib.listToAttrs (
    map (name: {
      inherit name;
      value = pkgs.callPackage (./${name}/default.nix) { };
    }) packageNames
  );

in
autoPackages
