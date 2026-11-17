{ lib, ... }:

let
  sustainedWatts = 37.5;
  burstWatts = 50;
  uw = w: toString (builtins.floor (w * 1000000));
  # pages that threw uncorrectable ECC on cpu2 (B1/B2, B5), from BERT records logged at boot
  badPages = [
    "0x18f1eec000"
    "0x124b050000"
  ];
in
{
  boot.kernelParams = map (p: "memmap=4K\$" + p) badPages;

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
