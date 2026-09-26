{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.loaner.guest;
  user = cfg.name;
  home = "/home/${user}";
  hashFile = "/persistent/loaner/guest.hash";

  setPassword = pkgs.writeShellScriptBin "loaner-set-password" ''
    set -eu
    export PATH=${
      lib.makeBinPath [
        pkgs.coreutils
        pkgs.mkpasswd
        pkgs.shadow
      ]
    }
    [ "$(id -un "$PKEXEC_UID")" = ${user} ] || exit 1
    hash=$(head -n 1 | mkpasswd -m yescrypt --stdin)
    usermod -p "$hash" ${user}
    install -m 600 /dev/null ${hashFile}.new
    printf '%s\n' "$hash" > ${hashFile}.new
    mv ${hashFile}.new ${hashFile}
  '';

  firstLogin = pkgs.writeShellScript "loaner-first-login" ''
    [ "$USER" = ${user} ] || exit 0
    ${pkgs.flatpak}/bin/flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    marker="$HOME/.local/state/loaner/password-set"
    [ -e "$marker" ] && exit 0
    kdialog=${pkgs.kdePackages.kdialog}/bin/kdialog
    while true; do
      pw=$($kdialog --title "Welcome" --newpassword "Welcome! Choose a password for this laptop. You'll use it to log in until the laptop is reset.") || continue
      [ -n "$pw" ] || continue
      if printf '%s\n' "$pw" | /run/wrappers/bin/pkexec ${setPassword}/bin/loaner-set-password; then
        mkdir -p "$(dirname "$marker")"
        touch "$marker"
        exit 0
      fi
    done
  '';

  resetPrompt = pkgs.writeShellScript "loaner-reset-prompt" ''
    ${pkgs.kdePackages.kdialog}/bin/kdialog --title "Reset laptop" --warningcontinuecancel \
      "This erases every file, app, setting and browser login on this laptop, then restarts it. It can't be undone." \
      --continue-label "Erase and restart" \
      && systemctl start --no-block loaner-reset.service
  '';

  policy = pkgs.writeTextDir "share/polkit-1/actions/in.hayl.loaner.policy" ''
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE policyconfig PUBLIC "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
      "http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd">
    <policyconfig>
      <action id="in.hayl.loaner.set-password">
        <description>Set the account password</description>
        <message>Set the password for this laptop's account</message>
        <defaults>
          <allow_any>auth_admin</allow_any>
          <allow_inactive>auth_admin</allow_inactive>
          <allow_active>auth_admin</allow_active>
        </defaults>
        <annotate key="org.freedesktop.policykit.exec.path">${setPassword}/bin/loaner-set-password</annotate>
      </action>
    </policyconfig>
  '';
in
{
  users.mutableUsers = false;
  users.users.haylin.enable = lib.mkForce false;

  users.users.${user} = {
    isNormalUser = true;
    description = "Me";
    inherit home;
    hashedPasswordFile = hashFile;
    extraGroups = [
      "networkmanager"
      "audio"
      "video"
    ];
  };

  system.activationScripts.loaner-guest-password.text = ''
    install -d -m 700 /persistent/loaner
    if [ ! -s ${hashFile} ]; then
      install -m 600 /dev/null ${hashFile}
      printf '%s\n' '${cfg.defaultHashedPassword}' > ${hashFile}
    fi
  '';
  system.activationScripts.users.deps = [ "loaner-guest-password" ];

  systemd.tmpfiles.rules = [ "d ${home} 0700 ${user} users -" ];

  systemd.paths.loaner-guest-hash = {
    wantedBy = [ "multi-user.target" ];
    pathConfig.PathChanged = "/etc/shadow";
  };
  systemd.services.loaner-guest-hash = {
    serviceConfig.Type = "oneshot";
    script = ''
      hash=$(${pkgs.gawk}/bin/awk -F: '$1 == "${user}" { print $2 }' /etc/shadow)
      case "$hash" in
        '$'*) [ "$hash" = "$(cat ${hashFile})" ] || printf '%s\n' "$hash" > ${hashFile} ;;
      esac
    '';
  };

  systemd.services.loaner-reset = {
    description = "Erase the loaner account";
    serviceConfig.Type = "oneshot";
    path = with pkgs; [
      btrfs-progs
      coreutils
      procps
      util-linux
      systemd
    ];
    script = ''
      systemctl stop display-manager.service
      loginctl terminate-user ${user} || true
      sleep 2
      pkill -KILL -u ${user} || true
      umount -R ${home} || umount -Rl ${home}

      mkdir -p /run/loaner-btrfs
      mount -o subvol=/ /dev/mapper/crypted /run/loaner-btrfs
      btrfs subvolume list -o /run/loaner-btrfs/guest | cut -f 9- -d ' ' | sort -r | while read -r sub; do
        btrfs subvolume delete "/run/loaner-btrfs/$sub"
      done
      btrfs subvolume delete --commit-after /run/loaner-btrfs/guest
      btrfs subvolume sync /run/loaner-btrfs
      btrfs subvolume create /run/loaner-btrfs/guest
      umount /run/loaner-btrfs

      printf '%s\n' '${cfg.defaultHashedPassword}' > ${hashFile}
      fstrim /persistent || true
      systemctl reboot
    '';
  };

  security.polkit.enable = true;
  security.polkit.enablePkexecWrapper = true;
  security.polkit.extraConfig = ''
    polkit.addRule(function (action, subject) {
      if (subject.user != "${user}") return;
      if (action.id == "in.hayl.loaner.set-password") return polkit.Result.YES;
      if (action.id == "org.freedesktop.systemd1.manage-units" &&
          action.lookup("unit") == "loaner-reset.service" &&
          action.lookup("verb") == "start") return polkit.Result.YES;
    });
  '';

  services.flatpak.enable = true;

  environment.etc."xdg/autostart/loaner-first-login.desktop".text = ''
    [Desktop Entry]
    Type=Application
    Name=Loaner first login
    Exec=${firstLogin}
    X-KDE-autostart-phase=2
    NoDisplay=true
  '';

  environment.systemPackages = [
    policy
    (pkgs.makeDesktopItem {
      name = "loaner-reset";
      desktopName = "Reset laptop";
      comment = "Erase this account and start fresh";
      icon = "edit-clear-all";
      exec = "${resetPrompt}";
      categories = [ "System" ];
    })
  ];

  nix.settings.allowed-users = [ "root" ];
}
