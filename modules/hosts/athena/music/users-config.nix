{
  groups = {
    media = {
      gid = 991;
    };
  };

  users = {
    alice = {
      uid = 2001;
      sshKeys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOdK5ssxU1XL5iOOJjQ27Plo4nFmS6df9GhkOYg1GJaT"
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL haylin@haytop"
      ];
    };
    haylin = {
      uid = 1000;
      fromHost = true; # Use existing host user, don't create on host
      # SSH keys will be inherited from host config
    };
  };
}
