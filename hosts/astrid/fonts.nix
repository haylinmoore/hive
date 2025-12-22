{ pkgs, hive, ... }:
{
  fonts = {
    packages = with pkgs; [
      noto-fonts
      noto-fonts-cjk-sans
      noto-fonts-color-emoji
      fira-code
      fira-code-symbols
      roboto
      nerd-fonts.fira-code
      nerd-fonts.droid-sans-mono
      julia-mono
    ];
    fontconfig.defaultFonts = {
      monospace = [
        "JuliaMono"
        "FiraCode Nerd Font"
      ];
    };
  };
}
