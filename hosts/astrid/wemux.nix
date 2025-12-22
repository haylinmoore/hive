{ pkgs, ... }:

{
  environment.etc."wemux.conf".text = ''
    host_list=(root hmoore)
    host_groups=(wheel wemux)
    allow_pair_mode="true"
    allow_server_change="true"
    default_server_name="haymux"
  '';
}
