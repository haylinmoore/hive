{
  pkgs,
  lib,
  hive,
  ...
}:

let
  sources = hive.sources;
  home-manager = import sources.home-manager { inherit pkgs; };

  # Build home-manager configurations
  homeConfigurations = {
    work = home-manager.lib.homeManagerConfiguration {
      inherit pkgs;
      modules = [ ./profiles/work.nix ];
      extraSpecialArgs = { inherit hive; };
    };
  };
in
{
  # Expose individual configs as derivations for nix-build -A home.work
  work = homeConfigurations.work.activationPackage;

  # Pass through for potential future use
  inherit homeConfigurations;
}
