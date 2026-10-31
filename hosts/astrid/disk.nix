{ pkgs, ... }:
{
  nix.gc = {
    automatic = true;
    options = "--delete-older-than 14d";
    randomizedDelaySec = "30min";
  };

  systemd.services.qumulo-free-disk = {
    description = "Trim ~hmoore/src build/tmp, linking cache and unheld build graphs";
    path = [
      "/run/wrappers"
      pkgs.nix
    ];
    serviceConfig = {
      Type = "oneshot";
      User = "hmoore";
      Nice = 19;
      IOSchedulingClass = "idle";
    };
    script = ''
      /home/hmoore/notes/bin/env --no-remote-fs exec \
        bash -c 'cd /home/hmoore/src && ./environment ai/tools/safely_free_disk_space.py'
    '';
  };

  systemd.timers.qumulo-free-disk = {
    wantedBy = [ "timers.target" ];
    timerConfig = {
      OnCalendar = "*-*-* 00/6:17:00";
      RandomizedDelaySec = "15m";
      Persistent = true;
    };
  };
}
