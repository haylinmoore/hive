{ pkgs, ... }:
pkgs.vimUtils.buildVimPlugin {
  pname = "qvim";
  version = "2025-03-23";
  # The tarball has no top-level directory, which builtins.fetchTarball rejects.
  src = pkgs.fetchzip {
    url = "https://gravyweb.eng.qumulo.com/home/amitha/vim.tar.gz";
    stripRoot = false;
    hash = "sha256-9ZX2NitOQ6RMvrRSct3zXGYFt8aeKdn4FD4CDYHzM1M=";
  };
}
