{ ... }:

let
  sustainedWatts = 37.5;
  burstWatts = 50;
  uw = w: toString (builtins.floor (w * 1000000));
in
{

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
