#!/usr/bin/env bash

cd "$(dirname "$0")" || exit
cmd=${1}

# hived's runner has a deliberately small PATH and no hostname binary,
# so read the name from the kernel instead of depending on a package.
this_host() {
  cat /proc/sys/kernel/hostname 2>/dev/null || hostname
}

usage() {
  echo "Usage: $0 {colmena|work|dns|arin|pepper|astrid|zoe|athena}"
  echo "  colmena - Deploy to all servers via colmena"
  echo "  work    - Deploy work home-manager configuration"
  echo "  dns     - Preview or push DNS records (preview|push [-y])"
  echo "  arin    - Show, preview or push ARIN rDNS, IRR and RPKI (status|preview|push)"
  echo "  pepper  - Deploy pepper NixOS configuration"
  echo "  astrid  - Deploy astrid NixOS configuration"
  echo "  zoe     - Build and activate zoe locally (build|activate|switch)"
  echo "  athena  - Build and activate athena locally (build|activate|switch)"
}

# Auto-detect if no command provided
if [[ -z "$cmd" ]]; then
  if [[ "$USER" == "hmoore" ]]; then
    cmd="work"
  else
    case "$(this_host)" in
      pepper | astrid | zoe | athena) cmd="$(this_host)" ;;
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

rebuild=("${sudo[@]}" nixos-rebuild)
if [[ $EUID -ne 0 ]] && command -v hive-rebuild > /dev/null 2>&1; then
  rebuild=(pkexec "$(readlink -f "$(command -v hive-rebuild)")")
fi

activate_toplevel() {
  # This switches the machine it runs on. Activating another host's closure
  # here would replace this system with that one.
  if [[ "$(this_host)" != "$cmd" ]]; then
    echo "refusing to activate $cmd on $(this_host)" >&2
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
  arin)
    shift
    action=${1:-preview}
    if [[ "$action" != status && "$action" != preview && "$action" != push ]]; then
      echo "Usage: $0 arin {status|preview|push [-y]}" >&2
      exit 1
    fi

    state=$(nix-build --no-out-link -A arin.state) || exit 1
    sync=$(nix-build --no-out-link -A arin.sync) || exit 1

    arin() {
      nix-shell --run "sops exec-env secrets/arin.env '$sync/bin/arin-sync $1 $state'"
    }

    if [[ "$action" == status ]]; then
      arin status
      exit
    fi

    arin preview || exit 1
    if [[ "$action" == push ]]; then
      if [[ "${2:-}" == -y ]]; then
        answer=y
      else
        read -rp "push these changes? [y/N] " answer
      fi
      [[ "$answer" == y ]] && arin push
    fi
    ;;
  dns)
    shift
    action=${1:-preview}
    if [[ "$action" != preview && "$action" != push ]]; then
      echo "Usage: $0 dns {preview|push [-y]}" >&2
      exit 1
    fi

    out=$(mktemp -d)
    trap 'rm -rf "$out"' EXIT
    run_colmena eval --config colmena.nix dns/default.nix > "$out/records.json" || exit 1
    cp dns/dnsconfig.js dns/creds.json "$out/"

    dnscontrol() {
      nix-shell --run "sops exec-env secrets/dns.env 'dnscontrol $1 --config $out/dnsconfig.js --creds $out/creds.json'"
    }

    dnscontrol preview || exit 1
    if [[ "$action" == push ]]; then
      if [[ "${2:-}" == -y ]]; then
        answer=y
      else
        read -rp "push these changes? [y/N] " answer
      fi
      [[ "$answer" == y ]] && dnscontrol push
    fi
    ;;
  pepper | astrid)
    shift
    action=${1:-switch}
    toplevel=".gcroots/node-$cmd"

    case "$action" in
      build) nix-build default.nix --attr "hosts.$cmd.system" --out-link "$toplevel" ;;
      activate) activate_toplevel ;;
      *)
        if [[ "$cmd" == pepper ]]; then
          "${rebuild[@]}" "$action" --file "$PWD/default.nix" --attr "hosts.$cmd"
        else
          "${sudo[@]}" nixos-rebuild "$action" --file default.nix --attr "hosts.$cmd"
        fi
        ;;
    esac
    ;;
  zoe | athena)
    # Build locally through colmena so the closure matches a colmena deploy
    # byte for byte, then activate it without going over ssh.
    shift
    action=${1:-switch}
    toplevel=".gcroots/node-$cmd"

    do_build() {
      run_colmena build --config colmena.nix --on "$cmd" --no-build-on-target --keep-result
    }

    case "$action" in
      build) do_build ;;
      activate) activate_toplevel ;;
      switch) do_build && activate_toplevel ;;
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
