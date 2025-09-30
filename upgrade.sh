#!/usr/bin/env bash

cd "$(dirname "$0")" || exit
cmd=${1}

# Auto-detect if no command provided
if [[ -z "$cmd" ]]; then
  if [[ "$USER" == "hmoore" ]]; then
    cmd="work"
  else
    echo "Usage: $0 {colmena|work}"
    echo "  colmena - Deploy to all servers via colmena"
    echo "  work    - Deploy work home-manager configuration"
    exit 1
  fi
fi

case "$cmd" in
  colmena)
    shift
    if [[ $# -eq 0 ]]; then
      nix-shell --run "colmena apply"
    else
      nix-shell --run "colmena $*"
    fi
    ;;
  work)
    nix-build -A home.work && ./result/activate
    ;;
  *)
    echo "Usage: $0 {colmena|work}"
    echo "  colmena - Deploy to all servers via colmena"
    echo "  work    - Deploy work home-manager configuration"
    exit 1
    ;;
esac