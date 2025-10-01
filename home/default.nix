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
    sasha = home-manager.lib.homeManagerConfiguration {
      inherit pkgs;
      modules = [ ./profiles/sasha ];
      extraSpecialArgs = { inherit hive; };
    };
  };
in
{
  # Expose individual configs as derivations for nix-build -A home.work
  work = homeConfigurations.work.activationPackage;
  sasha = homeConfigurations.sasha.activationPackage;

  # Pass through for potential future use
  inherit homeConfigurations;
}
