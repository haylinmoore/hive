{ ... }:

let
  astrid = "astrid.hive.hayl.in";
in
{
  nix.distributedBuilds = true;
  nix.settings.builders-use-substitutes = true;
  nix.buildMachines = [
    {
      hostName = astrid;
      protocol = "ssh-ng";
      sshUser = "nix-ssh";
      sshKey = "/etc/ssh/ssh_host_ed25519_key";
      systems = [ "x86_64-linux" ];
      maxJobs = 8;
      speedFactor = 8;
      supportedFeatures = [
        "nixos-test"
        "benchmark"
        "big-parallel"
        "kvm"
      ];
    }
  ];

  programs.ssh.knownHosts.${astrid}.publicKey =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKsB4vijLXHBvZVNL9cu9VMJ7K4ODLtIXWgOFiI0YJj5";

  # A switched off astrid would otherwise stall builds for a full TCP timeout.
  programs.ssh.extraConfig = ''
    Host ${astrid}
      ConnectTimeout 5
  '';
}
