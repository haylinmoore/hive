{
  config,
  lib,
  pkgs,
  ...
}:

let
  confirmPage = pkgs.writeTextDir "index.html" (builtins.readFile ./index.html);
  manageKeys = pkgs.writeText "door-keys.py" (builtins.readFile ./door-keys.py);
  python = "${pkgs.python3}/bin/python3";
  keysDir = "/var/lib/door-keys";
  keysFile = "${keysDir}/door_keys.json";
in
{
  users.groups.doorkeys = { };
  users.users.hass.extraGroups = [ "doorkeys" ];
  users.users.nginx.extraGroups = [ "doorkeys" ];

  systemd.tmpfiles.rules = [
    "d ${keysDir} 2770 hass doorkeys -"
  ];

  sops.secrets."doorking-code" = {
    sopsFile = ../../../secrets/zoe/doorking-code.yaml;
    key = "code";
    owner = "nginx";
  };

  services.nginx.package = pkgs.openresty;

  services.home-assistant = {
    customLovelaceModules = [
      pkgs.home-assistant-custom-lovelace-modules.mushroom
    ];

    lovelaceConfig = {
      title = "Keys";
      views = [
        {
          type = "sections";
          title = "Keys";
          sections = [
            {
              cards = [
                {
                  type = "heading";
                  heading = "Lock State";
                  heading_style = "title";
                  icon = "mdi:door-closed-lock";
                  grid_options.rows = 1;
                }
                {
                  type = "custom:mushroom-lock-card";
                  entity = "lock.level_bolt_matter";
                  fill_container = true;
                  layout = "default";
                  grid_options.columns = "full";
                }
                {
                  type = "heading";
                  heading = "Keys";
                  heading_style = "title";
                  icon = "mdi:key";
                  badges = [ ];
                  grid_options = {
                    columns = "full";
                    rows = 1;
                  };
                }
                {
                  type = "markdown";
                  content = ''
                    {% set keys = state_attr('sensor.door_keys', 'keys') or [] %}
                    {% if keys | length == 0 %}
                      *No active keys.*
                    {% else %}
                    {% for key in keys | sort %}
                    - [**{{ key.split(':')[0] }}**](https://ha.uwu.estate/door?secret={{ key }})
                    {% endfor %}
                    {% endif %}
                  '';
                }
                {
                  type = "heading";
                  heading = "Key Management";
                  heading_style = "title";
                  icon = "mdi:key-chain";
                  badges = [ ];
                  grid_options = {
                    columns = "full";
                    rows = 1;
                  };
                }
                {
                  type = "custom:mushroom-entity-card";
                  entity = "script.generate_door_key";
                  name = "Create New Door Key";
                  icon = "mdi:key-plus";
                  icon_color = "green";
                  icon_type = "icon";
                  fill_container = true;
                  secondary_info = "none";
                  tap_action.action = "more-info";
                  hold_action.action = "none";
                  double_tap_action.action = "none";
                  grid_options = {
                    columns = 6;
                    rows = 1;
                  };
                }
                {
                  type = "custom:mushroom-entity-card";
                  entity = "script.delete_door_key";
                  name = "Delete Door Key";
                  icon = "mdi:key-remove";
                  icon_color = "red";
                  icon_type = "icon";
                  secondary_info = "none";
                  tap_action.action = "more-info";
                  grid_options = {
                    columns = 6;
                    rows = 1;
                  };
                }
              ];
            }
          ];
        }
      ];
    };

    config = {
      homeassistant.allowlist_external_dirs = [ keysDir ];

      lovelace.dashboards.nixos-lovelace = {
        mode = "yaml";
        filename = "ui-lovelace.yaml";
        title = "Keys";
        icon = "mdi:key";
        show_in_sidebar = true;
      };

      shell_command = {
        door_key_add = ''${python} ${manageKeys} ${keysFile} add "{{ key_name }}"'';
        door_key_remove = ''${python} ${manageKeys} ${keysFile} remove "{{ key_name }}"'';
        door_key_list = "${python} ${manageKeys} ${keysFile} list";
      };

      command_line = [
        {
          sensor = {
            name = "Door Keys";
            command = "${python} ${manageKeys} ${keysFile} list";
            value_template = "{{ value_json['keys'] | length }}";
            json_attributes = [ "keys" ];
            scan_interval = 86400;
          };
        }
      ];

      "script door" = {
        generate_door_key = {
          alias = "Generate Door Key";
          icon = "mdi:key-plus";
          mode = "single";
          fields.key_name = {
            name = "Key Name";
            description = "A label so you know whose key this is (spaces/colons are stripped).";
            required = true;
            selector.text = { };
          };
          sequence = [
            {
              action = "shell_command.door_key_add";
              data.key_name = "{{ key_name }}";
            }
            {
              action = "homeassistant.update_entity";
              target.entity_id = "sensor.door_keys";
            }
          ];
        };

        delete_door_key = {
          alias = "Delete Door Key";
          icon = "mdi:key-remove";
          mode = "single";
          fields.key_name = {
            name = "Key Name";
            description = "The label of the key to remove.";
            required = true;
            selector.text = { };
          };
          sequence = [
            {
              action = "shell_command.door_key_remove";
              data.key_name = "{{ key_name }}";
            }
            {
              action = "homeassistant.update_entity";
              target.entity_id = "sensor.door_keys";
            }
          ];
        };
      };

      "automation door" = [
        {
          alias = "Door Key Access";
          triggers = [
            {
              trigger = "webhook";
              webhook_id = "door";
              allowed_methods = [ "POST" ];
              local_only = false;
            }
          ];
          actions = [
            {
              action = "shell_command.door_key_list";
              response_variable = "kr";
            }
            {
              variables = {
                keys = "{{ (kr.stdout | from_json)['keys'] if kr.returncode == 0 else [] }}";
                secret = "{{ trigger.data.get('secret') if trigger.data is mapping else none }}";
                key_name = "{{ (trigger.data.get('secret', '') if trigger.data is mapping else '').split(':')[0] }}";
              };
            }
            {
              condition = "template";
              value_template = "{{ secret and secret in keys }}";
            }
            {
              choose = [
                {
                  conditions = [
                    {
                      condition = "state";
                      entity_id = "input_boolean.forcefield";
                      state = "on";
                    }
                  ];
                  sequence = [
                    {
                      action = "logbook.log";
                      data = {
                        name = "Door";
                        entity_id = "lock.level_bolt_matter";
                        message = "unlock by {{ key_name }} blocked (forcefield on)";
                      };
                    }
                    {
                      action = "notify.mobile_app_siprnet";
                      data = {
                        title = "Door Blocked";
                        message = "{{ key_name }} tried to unlock but the forcefield was on";
                      };
                    }
                  ];
                }
              ];
              default = [
                {
                  action = "lock.unlock";
                  target.entity_id = "lock.level_bolt_matter";
                }
                {
                  action = "logbook.log";
                  data = {
                    name = "Door";
                    entity_id = "lock.level_bolt_matter";
                    message = "unlocked by {{ key_name }}";
                  };
                }
                {
                  action = "notify.mobile_app_siprnet";
                  data = {
                    title = "Door Unlocked";
                    message = "{{ key_name }} unlocked the door";
                  };
                }
              ];
            }
          ];
          mode = "parallel";
        }
      ];
    };
  };

  services.nginx.virtualHosts."ha.uwu.estate".locations."= /door" = {
    extraConfig = ''
      default_type text/html;
      charset utf-8;
      content_by_lua_block {
        local function slurp(p)
          local f = io.open(p, "r")
          if not f then return nil end
          local d = f:read("*a")
          f:close()
          return d
        end

        local cjson = require("cjson.safe")
        local secret = ngx.req.get_uri_args().secret or ""
        local keys = cjson.decode(slurp("${keysFile}") or "[]") or {}

        local valid = false
        for _, k in ipairs(keys) do
          if k == secret then valid = true break end
        end

        if not valid then
          return ngx.exit(ngx.HTTP_NOT_FOUND)
        end

        local code = (slurp("/run/secrets/doorking-code") or ""):gsub("%s+$", "")
        local html = slurp("${confirmPage}/index.html") or ""
        ngx.print((html:gsub("@DOORKING_CODE@", function() return code end)))
      }
    '';
  };
}
