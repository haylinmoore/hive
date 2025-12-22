{
  hosts = { };

  hosts."all" = {
    setEnv = {
      "TERM" = "xterm-256color";
    };
    match = "Host *";
  };

  hosts."wolfgirls" = {
    match = "Host *.wolfgirl.systems";
    user = "haylin";
    forwardAgent = true;
  };

  hosts."devhack" = {
    host = "dev.hack.seattle.wa.us";
    user = "haylin";
  };

  hosts."starguess.ing" = {
    user = "debian";
    host = "startguess.ing";
    port = 2222;
    hostname = "jump.startguess.ing";
  };

  hosts."cs367-jump" = {
    user = "haylin";
    host = "jump.startguess.ing";
  };

  hosts."hack" = {
    user = "dean";
    host = "hack";
    hostname = "22.0.0.2";
    proxyJump = "jump.startguess.ing";
  };

  # 9ty9
  hosts."alyx.9ty9.net" = {
    user = "root";
  };
  hosts."zoe.9ty9.net" = {
    user = "root";
  };

  hosts."infra.mkr.cx" = {
    user = "maker";
    match = "Host *.infra.mkr.cx";
    proxyJump = "elnux.cs.umass.edu";
  };

  # UMass
  hosts."all_umass" = {
    match = "Host *.umass.edu";
    user = "hrmoore";
  };

  hosts."gaia.cs.umass.edu" = {
    proxyJump = "elnux.cs.umass.edu";
  };
}
