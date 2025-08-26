{
  config,
  lib,
  pkgs,
  ...
}:

{
  environment.persistence."/persistent".directories = [ "/var/lib/private/soft-serve" ];

  services.soft-serve = {
    enable = true;
    settings = {
      name = "haylin's repos";
      log_format = "text";
      ssh = {
        listen_addr = ":2222";
        public_url = "ssh://soft.hayl.in:2222";
        max_timeout = 30;
        idle_timeout = 120;
      };
      http = {
        listen_addr = "127.0.0.1:15642";
        public_url = "https://soft.hayl.in";
      };
      initial_admin_keys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHavg+rhFmR2p9wuWiO4VxKaIXpq1gOm17jCoZ9jMxvL me@haylinmoore.com"
      ];
    };
  };

  proxySites.soft = {
    domain = "soft.hayl.in";
    proxyUri = "http://127.0.0.1:15642/";
    useACMEHost = "hayl.in";
  };
}
