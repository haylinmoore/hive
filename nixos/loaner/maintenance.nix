{
  config,
  lib,
  pkgs,
  ...
}:
let
  hashFile = "/persistent/loaner/guest.hash";
in
{
  services.openssh.settings = {
    PasswordAuthentication = false;
    KbdInteractiveAuthentication = false;
  };
  systemd.services.sshd.wantedBy = lib.mkForce [ ];

  systemd.services.loaner-remote-access = {
    description = "Allow SSH only while the laptop is unclaimed";
    wantedBy = [ "multi-user.target" ];
    serviceConfig.Type = "oneshot";
    path = [
      pkgs.coreutils
      pkgs.gawk
      config.systemd.package
    ];
    script = ''
      if [ "$(cat ${hashFile} 2>/dev/null)" = '${config.loaner.guest.defaultHashedPassword}' ]; then
        systemctl start sshd.service
      else
        systemctl stop sshd.service
        loginctl list-sessions --no-legend | awk '{ print $1 }' | while read -r id; do
          if [ "$(loginctl show-session "$id" -p Remote --value)" = yes ]; then
            loginctl terminate-session "$id"
          fi
        done
      fi
    '';
  };

  systemd.paths.loaner-remote-access = {
    wantedBy = [ "multi-user.target" ];
    pathConfig.PathChanged = hashFile;
  };

  networking.firewall.checkReversePath = "loose";
}
