{ config, pkgs, ... }:
let
  hiveRebuild = pkgs.writeShellScriptBin "hive-rebuild" ''
    export PATH=/run/current-system/sw/bin
    exec nixos-rebuild "$@"
  '';

  policy = pkgs.writeTextDir "share/polkit-1/actions/in.hayl.hive.rebuild.policy" ''
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
      "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
    <policyconfig>
      <action id="in.hayl.hive.rebuild">
        <description>Rebuild ${config.networking.hostName} from hive</description>
        <message>Rebuild and switch ${config.networking.hostName} to the configuration in ~/hive</message>
        <defaults>
          <allow_any>auth_admin</allow_any>
          <allow_inactive>auth_admin</allow_inactive>
          <allow_active>auth_admin</allow_active>
        </defaults>
        <annotate key="org.freedesktop.policykit.exec.path">${hiveRebuild}/bin/hive-rebuild</annotate>
      </action>
    </policyconfig>
  '';
in
{
  security.polkit.enablePkexecWrapper = true;
  environment.systemPackages = [
    hiveRebuild
    policy
  ];
}
