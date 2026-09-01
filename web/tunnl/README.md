# tunnl

SSH reverse-tunnel server. Open a remote forward and the server proxies
`https://<subdomain>.<domain>` back to your local port:

```
ssh -t -R 80:localhost:8080 tunnl.hayl.in
```

Pass a name as the SSH command to pick the subdomain yourself:

```
ssh -t -R 80:localhost:8080 tunnl.hayl.in myapp
```

Without one you get a generated `adjective-noun-hex` name. A name already
in use, or one that is not a single lowercase DNS label, comes back as an
error. The tunnel lives as long as the SSH connection.

Only keys listed in `AUTHORIZED_KEYS` may connect. On athena that is
root's key list from `nixos/shared/users.nix`.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SSH_ADDR` | `:22` | SSH listener |
| `HTTP_ADDR` | `:80` | HTTP listener, redirects to HTTPS |
| `HTTPS_ADDR` | `:443` | HTTPS listener |
| `STATS_ADDR` | `127.0.0.1:9090` | Stats endpoint, loopback only |
| `HOST_KEY_PATH` | `host_key` | SSH host key, generated if absent |
| `AUTHORIZED_KEYS` | `authorized_keys` | Keys permitted to open tunnels |
| `TLS_CERT` / `TLS_KEY` | Let's Encrypt paths | Wildcard cert for `*.<domain>`, re-read when it changes |
| `DOMAIN` | `tunnl.gg` | Domain tunnels are served under |

`GET /?tunnels=true` on the stats endpoint lists the active subdomains
with the client IP and start time of each.

## Deployment

`hosts/athena/tunnl.nix` runs this in a nixos-container on its own public
address, since it needs 22, 80 and 443 and athena already binds all three.

## Provenance

Derived from [tunnl.gg](https://github.com/klipitkas/tunnl.gg) at 41003ea,
MIT licensed, see LICENSE. Diverged substantially: this instance is
single-tenant and key-restricted, so the abuse tracking, IP blocking, rate
limits, transfer caps, tunnel expiry and browser interstitial that upstream
needs for public hosting are all gone.
