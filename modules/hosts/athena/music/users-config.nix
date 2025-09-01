{
  groups = {
    media = {
      gid = 991;
    };
  };

  users = {
    haylin = {
      uid = 1000;
      fromHost = true; # Use existing host user, don't create on host
      # SSH keys will be inherited from host config
    };
    alice = {
      uid = 2001;
      sshKeys = [
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOdK5ssxU1XL5iOOJjQ27Plo4nFmS6df9GhkOYg1GJaT"
      ];
    };
    zoefiri = {
      uid = 2002;
      sshKeys = [
        "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDI9mMC+TRRmGOtjcwfRMKVhKuJbSl6BX8c9F+wpgCoanoTXhuKIS+aPzZgqBm4McnotgP8q7NTo/UQjJcf7uw7xNbt6pKqVA+buvTEXycCLtgvyk9jS+UwczQnrLotzYCzH35XA2X7rFeAcp09mN0Y9NgvQWPTUXzrUj4e55P1uGu1rkhRDeKTjOdHPnKmTffjy49zt5CjF7f9Qq+dXllErsU5p2y3NUNg5jDcBfEYKboZJRFqUTCVcvTp6CMc3cVrOUgrQkTdAT+Z2ynQmZIrdrZcgGjTjM+jfkC7FOcZ9PFb5odPGL3qvLygX7F80d7mxxJE8MjOdwiyfgGgx/U8kDQSSgJgvTwEhT2pTc3Z/3si0otqHbf/tuormmnf1u5zqDTZnkYK78bW0Z12XUucyN1wVlsaz26OV0lhhTpAyCSZDau6eZBFP/M9Svxhza4zheS48utwjRluAiit226AA3ERu9S9hlkgiY0huvt2GH1vl0zINhGez51uH7P2KAEAy4OZNF3QcPQW68NE3SqTJ4Yjoc1lwlapqAqPOTeR45LgjS9VswR7+CAROE3nYhXrCtLYjEdOrODNwp+vk/rv0g+IFOdFO8cZdcFJgX8YRBQtv7OAKvh8Hp5NnXgwXgZAin+JmOzh03WLIdSQYXCU2wRs0G7qJCU49pE+NyiORw=="
      ];
    };
  };
}
