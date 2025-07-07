{
  config,
  lib,
  pkgs,
  ...
}:

{
  services.mysql = {
    enable = true;
    package = pkgs.mariadb;
  };
}
