{ lib, ... }:

let
  sustainedWatts = 37.5;
  burstWatts = 50;
  uw = w: toString (builtins.floor (w * 1000000));
in
{
  # B1/B2 (cpu2) throw uncorrectable ECC; keep the kernel off cpu2's whole range until B1 is swapped
  boot.kernelParams = [ "memmap=64G$0x1080000000" ];

  systemd.services.rapl-limits = {
    description = "RAPL package power limits";
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
    };
    script = ''
      for z in /sys/class/powercap/intel-rapl:?; do
        echo ${uw sustainedWatts} > $z/constraint_0_power_limit_uw
        echo ${uw burstWatts} > $z/constraint_1_power_limit_uw
      done
    '';
  };
}
