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
      hive.pkgs.berkeley-mono
      julia-mono
    ];
    fontconfig.defaultFonts = {
      monospace = [
        "Berkeley Mono"
        "JuliaMono"
      ];
    };
    fontconfig.localConf = ''
      <?xml version="1.0"?>
      <!DOCTYPE fontconfig SYSTEM "fonts.dtd">
      <fontconfig>
        <match target="pattern">
          <test qual="any" name="family" compare="eq"><string>Berkeley Mono</string></test>
          <edit name="family" mode="assign" binding="same"><string>JuliaMono</string></edit>
        </match>
      </fontconfig>
    '';
  };
}
