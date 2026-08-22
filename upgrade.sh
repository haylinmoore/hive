#!/usr/bin/env bash

cd "$(dirname "$0")" || exit
cmd=${1}

usage() {
  echo "Usage: $0 {colmena|work|sasha|astrid|zoe|athena}"
  echo "  colmena - Deploy to all servers via colmena"
  echo "  work    - Deploy work home-manager configuration"
  echo "  sasha   - Deploy sasha NixOS configuration"
  echo "  astrid  - Deploy astrid NixOS configuration"
  echo "  zoe     - Build and activate zoe locally (build|activate|switch)"
  echo "  athena  - Build and activate athena locally (build|activate|switch)"
}

# Auto-detect if no command provided
if [[ -z "$cmd" ]]; then
  if [[ "$USER" == "hmoore" ]]; then
    cmd="work"
  else
    case "$(hostname)" in
      sasha | astrid | zoe | athena) cmd="$(hostname)" ;;
      *)
        usage
        exit 1
        ;;
    esac
  fi
fi

# hived runs the activate step as root, where sudo is only an extra dependency.
sudo=(sudo)
if [[ $EUID -eq 0 ]]; then
  sudo=()
fi

# Run colmena from the environment if present, otherwise from the dev shell.
run_colmena() {
  if command -v colmena > /dev/null 2>&1; then
    colmena "$@"
  else
    nix-shell --run "colmena $*"
  fi
}

case "$cmd" in
  colmena)
    shift
    if [[ $# -eq 0 ]]; then
      nix-shell --run "colmena apply --config colmena.nix"
    else
      nix-shell --run "colmena --config colmena.nix $*"
    fi
    ;;
  work)
    nix-build -A home.work && ./result/activate
    ;;
  sasha)
    shift
    action=${1:-switch}
    "${sudo[@]}" nixos-rebuild "$action" --file default.nix --attr hosts.sasha
    ;;
  astrid)
    shift
    action=${1:-switch}
    "${sudo[@]}" nixos-rebuild "$action" --file default.nix --attr hosts.astrid
    ;;
  zoe | athena)
    # Build locally through colmena so the closure matches a colmena deploy
    # byte for byte, then activate it without going over ssh.
    shift
    action=${1:-switch}
    toplevel=".gcroots/node-$cmd"

    do_build() {
      run_colmena build --config colmena.nix --on "$cmd" --no-build-on-target
    }

    do_activate() {
      # This switches the machine it runs on. Activating another host's closure
      # here would replace this system with that one.
      if [[ "$(hostname)" != "$cmd" ]]; then
        echo "refusing to activate $cmd on $(hostname)" >&2
        echo "run this on $cmd, or from here use '$0 colmena apply --on $cmd'" >&2
        exit 1
      fi
      if [[ ! -L "$toplevel" ]]; then
        echo "$toplevel is missing, run '$0 $cmd build' first" >&2
        exit 1
      fi
      "${sudo[@]}" nix-env -p /nix/var/nix/profiles/system --set "$(readlink -f "$toplevel")"
      "${sudo[@]}" /nix/var/nix/profiles/system/bin/switch-to-configuration switch
    }

    case "$action" in
      build) do_build ;;
      activate) do_activate ;;
      switch) do_build && do_activate ;;
      *)
        echo "Usage: $0 $cmd {build|activate|switch}" >&2
        exit 1
        ;;
    esac
    ;;
  *)
    usage
    exit 1
    ;;
esac
