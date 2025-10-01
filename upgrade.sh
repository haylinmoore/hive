#!/usr/bin/env bash

cd "$(dirname "$0")" || exit
cmd=${1}

# Auto-detect if no command provided
if [[ -z "$cmd" ]]; then
  if [[ "$USER" == "hmoore" ]]; then
    cmd="work"
  elif [[ "$(hostname)" == "sasha" ]]; then
    cmd="sasha"
  else
    echo "Usage: $0 {colmena|work|sasha}"
    echo "  colmena - Deploy to all servers via colmena"
    echo "  work    - Deploy work home-manager configuration"
    echo "  sasha   - Deploy sasha NixOS configuration"
    exit 1
  fi
fi

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
    sudo nixos-rebuild "$action" --file default.nix --attr hosts.sasha
    ;;
  *)
    echo "Usage: $0 {colmena|work|sasha}"
    echo "  colmena - Deploy to all servers via colmena"
    echo "  work    - Deploy work home-manager configuration"
    echo "  sasha   - Deploy sasha NixOS configuration"
    exit 1
    ;;
esac