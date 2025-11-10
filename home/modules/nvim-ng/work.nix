# Work-specific neovim configuration
{
  config,
  pkgs,
  hive,
  ...
}:
{
  imports = [
    ./plugins.nix
  ];

  programs.neovim = {
    defaultEditor = true;
    enable = true;
    extraConfig = builtins.readFile ./shared.vim + "\n" + builtins.readFile ./work.vim;
    extraPackages = [ pkgs.fzf ];
    vimAlias = true;
    vimdiffAlias = true;

    plugins = with pkgs.vimPlugins; [
      hive.pkgs.qvim
    ];
  };

  programs.fzf = {
    enable = true;
    defaultCommand = "${pkgs.fd}/bin/fd --type f --ignore-file .hgignore";
    fileWidgetCommand = "${pkgs.fd}/bin/fd --type f --ignore-file .hgignore";
  };
}
