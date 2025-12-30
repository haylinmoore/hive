{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.defaultHttp;

  defaultPage = pkgs.writeTextDir "index.html" cfg.content;

  hostname = config.networking.hostName;

  baseConfig = {
    http2 = true;
    http3 = true;
    quic = true;
    extraConfig = ''
      add_header Alt-Svc 'h3=":443"; ma=86400';
      set $server_hostname "${hostname}";
    '';
    locations."/" = {
      root = "${defaultPage}";
      extraConfig = ''
        ssi on;
        try_files $uri /index.html =404;
      '';
    };
  };

  # Generate HTTPS virtualHosts for each ACME host
  httpsHosts = lib.listToAttrs (
    map (acmeHost: {
      name = "default-https-${acmeHost}";
      value = baseConfig // {
        serverName = "*.${acmeHost}";
        useACMEHost = acmeHost;
        onlySSL = true;
      };
    }) cfg.acmeHosts
  );

in

{
  options.defaultHttp = {
    enable = lib.mkEnableOption "default HTTP catch-all server";

    acmeHosts = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = "List of ACME certificate hosts to create HTTPS defaults for.";
      example = [
        "hayl.in"
      ];
    };

    content = lib.mkOption {
      type = lib.types.lines;
      description = "HTML content to serve as the default page.";
      default = ''
        <!DOCTYPE html />
        <html>
            <head>
                <title>404 domain not found</title>
                <meta name="viewport" content="width=device-width, initial-scale=1">
          <style>
            body
            {
              background: linear-gradient(135deg, #ffeef8 0%, #f0e6ff 50%, #e6f0ff 100%);
              background-attachment: fixed;
              font-family: sans-serif;
              margin: 16px;
              min-height: 100vh;
            }

            main
            {
              padding: 16px;
              padding-top: 4px;
              padding-bottom: 32px;
              margin: auto;
              left: 0;
              right: 0;
              max-width: 568px;

              background-color: #fffcfd;
              border-radius: 12px;
              box-shadow: 0 4px 20px rgba(100, 50, 150, 0.2);

              background-image: url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAIAAAAYCAYAAADQ+yzZAAAANUlEQVQIW2P8L6f2nwEIGPEw/k2Rg6jBw/j/nwFqDm4G216oGjwMBqkZEHPwMMoYOiBqcDMAiYg1kSQlv4gAAAAASUVORK5CYII=");

              background-repeat: repeat-x;
              background-position:center bottom;
              image-rendering: pixelated;
              image-rendering: -moz-crisp-edges;
              image-rendering: crisp-edges;
            }

            main {
              position: relative;
            }

            h1 {
              text-align: center;
              color: #444;
            }

            p {
              text-align: center;
              color: #444;
            }

            dl {
              display: grid;
              grid-template-columns: auto 1fr;
              gap: 4px 12px;
              margin: 16px 0;
              font-size: 14px;
              color: #444;
            }

            dt {
              font-weight: bold;
              text-align: right;
            }

            dd {
              margin: 0;
              word-break: break-all;
            }

            hr {
              border: none;
              height: 4px;
              margin: 16px 0;
              background: linear-gradient(135deg, #ffeef8 0%, #f0e6ff 50%, #e6f0ff 100%);
              background-attachment: fixed;
              border-radius: 2px;
            }

          </style>
            </head>
            <body>
                <main>
                    <h1><!--# echo var="host" --> not found</h1>
                    <p>You've connected to hive node <strong><!--# echo var="server_hostname" --></strong></p>
                    <hr>
                    <dl>
                      <dt>Request</dt>
                      <dd><!--# echo var="request_method" --> <!--# echo var="request_uri" --></dd>
                      <dt>Protocol</dt>
                      <dd><!--# echo var="scheme" --> (<!--# echo var="server_protocol" -->)</dd>
                      <dt>Your IP</dt>
                      <dd><!--# echo var="remote_addr" --></dd>
                      <dt>Server</dt>
                      <dd><!--# echo var="server_addr_formatted" -->:<!--# echo var="server_port" --></dd>
                      <dt>Time</dt>
                      <dd><!--# echo var="date_local" --></dd>
                    </dl>
                </main>

            </body>
        </html>
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    services.nginx.enable = true;
    services.nginx.appendHttpConfig = ''
      map $server_addr $server_addr_formatted {
        ~:      [$server_addr];
        default $server_addr;
      }
    '';
    services.nginx.virtualHosts = {
      # Plain HTTP default for any domain
      "default-http" = baseConfig // {
        default = true;
        serverName = "_";
      };
    }
    // httpsHosts;
  };
}
