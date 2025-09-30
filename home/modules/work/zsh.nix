{ config, pkgs, ... }:

{
  programs.zsh = {
    enable = true;
    oh-my-zsh = {
      enable = true;
      theme = "robbyrussell";
      plugins = [ "git" ];
    };

    shellAliases = {
      srcd = "cd ~/src/";
      cr = "~/src/check_run.py";
      linta = "~/src/lint/all -a";
      lintac = "~/src/lint/all -ac";
      lintc = "~/src/lint/all -c";
      qimgcp = "cp build/debug/install/qinstall.qimg /mnt/gravytrain/home/hmoore/qinstall.qimg";
      szshrc = "source ~/.zshrc";
      b = "build --linking-cache rw --linking-cache-directory $PWD/build/.qonstruct/cache/";
      debug = "less ~/src/build/tmp/latest/**/debug.log";
      simkill = "tools/kill_stale_simnodes.sh";
      sup = "simnode/qc create -n 4 && export SIMNODE_REST_PORT=$(simnode/qc status --rest)";
      sdown = "simnode/qc cleanup";
    };

    initContent = ''
      # Nix daemon profile
      if [ -e '/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh' ]; then
        . '/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh'
      fi
            
      cptags() {
        cp -f /mnt/gravytrain/build/latest/src/{tags,TAGS} ~/src "$@"
      }

      fixssh() {
        eval $(tmux show-env -s |grep '^SSH_')
      }
    '';

    sessionVariables = {
      PATH = "$HOME/.nix-profile/bin:/opt/qumulo/toolchain/bin:$PATH";
      TERM = "xterm-256color";
      EDITOR = "nvim";
      VISUAL = "nvim";
    };
  };
}
