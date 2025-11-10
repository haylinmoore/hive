{ pkgs, ... }:
pkgs.vimUtils.buildVimPlugin {
  pname = "qvim";
  version = "2025-03-23";
  src = fetchTarball {
    url = "https://gravyweb.eng.qumulo.com/home/amitha/vim.tar.gz";
    sha256 = "sha256:0lrkyf0hs0iy2kwdjacyqsvharjwygfp4lmlpr6a8hsf5cvgd5gm";
  };
}
