# startguess.ing CT honeypot

Measure how quickly, and for how long, scanners come looking for a hostname after its certificate appears in Certificate Transparency logs.

## Idea

- Every day a new secret subdomain of startguess.ing gets exactly one certificate: a 6-day Let's Encrypt `shortlived` cert that is never renewed. Its private key is never written to disk and is thrown away right after issuance.
- A `*.startguess.ing` wildcard cert serves every name forever, so traffic to old names can still be recorded without reissuing anything.
- Every request to any name or address is recorded, so it can be lined up against when that name's cert was ordered and when it appeared in CT (certs.hayl.in already records the CT side).

## Names

- Day number: UTC days since the unix epoch.
- Each day has one name, which gets a cert and so appears in CT.
- The day number (at index `2 * day`, a leftover of a retired second name per day that keeps issued names stable) goes through a keyed 4-round Feistel permutation (HMAC-SHA256 round function), with cycle-walking to stay inside 2048³, and comes out as three BIP39 words, e.g. `east-visual-scout.startguess.ing`.
- The mapping is a bijection, so names are unique and decode back to their day. Without the key, future names can't be predicted.
- Implemented and tested in `src/names.rs`. CLI: `honeypot name [YYYY-MM-DD]` and `honeypot decode <name|address>`.


## Addresses

IPv4, 23.144.156.224/27:

| address | role |
|---|---|
| .224 | ns1, apex, wildcard and unknown names |
| .225 | ns2 |
| .226 | never assigned: background noise baseline |
| .227 to .255 | daily name rotation: day n uses .227 + (n mod 29), unambiguous for 29 days |

IPv6, 2602:fbf5:3:c7::/64:

| address | role |
|---|---|
| `2602:fbf5:3:c7::` | ns1, apex, wildcard |
| `2602:fbf5:3:c7::53` | ns2 |
| `2602:fbf5:3:c7::1` | noise baseline |
| `2602:fbf5:3:c7:<keyed 64-bit id>` | one per day, never reused |

The v6 interface id comes from a second keyed 64-bit Feistel permutation of the same index, so it can't be enumerated and decodes back to its day. Raw traffic with no SNI or Host can therefore be attributed by destination address: exactly on v6, within 29 days on v4.

## DNS

- startguess.ing is delegated to `ns1.startguess.ing` and `ns2.startguess.ing`, with glue at Porkbun (done, see Status).
- The honeypot is the authoritative server for the zone, on port 53 of its addresses, UDP and TCP.
- Answers are computed from the key and the clock, not stored records:
  - apex: SOA, NS, A/AAAA to .224 and `…:c7::`, CAA allowing only letsencrypt.org, `v=spf1 -all`
  - `ns1`/`ns2`: their addresses
  - a daily name: its rotating v4 address and unique v6 address once its cert has been ordered, the wildcard addresses before that
  - anything else under the zone: the wildcard addresses
  - `_acme-challenge` TXT: served from memory while an ACME order is pending
- No recursion (REFUSED), no AXFR, no DNSSEC, so the zone can't be walked or transferred.
- Every query is logged: time, resolver, name, type, transport, and EDNS Client Subnet when present.
- `dns/zones/startguess.ing.nix` has provider `honeypot`, so dnscontrol skips it, but the zone stays in the certs.hayl.in watchlist.

## Certificates

- ACME client inside the binary (instant-acme, ring backend).
- Wildcard plus apex: DNS-01 through the honeypot's own DNS, renewed normally. The only private key kept on disk.
- Daily cert: ordered at a keyed pseudo-random time each day, `shortlived` profile, DNS-01. Key generated in memory and dropped after the order finishes.
- Recorded per day: name, order start, issue time, serial, fingerprint, not_before/not_after, SCT timestamps, PEM.
- Let's Encrypt's own validation traffic is tagged so it isn't counted as hits.

## Capture

- All TCP and UDP ports on both prefixes reach the binary through nftables TPROXY. The original destination address and port come from the socket.
- TCP connections: peek the first bytes and identify the protocol.
  - TLS: record SNI, ALPN and a JA4 fingerprint, complete the handshake with the wildcard cert on any port, then handle what's inside.
  - HTTP and HTTPS: answer like a fresh nginx install (welcome page on `/`, nginx 404 elsewhere) and log method, path, query, headers and a capped body.
  - Other protocols: identify from first bytes or port, store the first 4 KB.
- UDP: log the first datagram per source and port.
- Protection is per connection, not a global cap: first-byte timeout around 5 s, idle timeout around 30 s, read caps, a high `LimitNOFILE`, a disk quota that prunes raw payloads first.
- Names that were never created (`www`, the apex, guesses) are logged with their real label as "unknown". Names outside the zone are logged as "foreign".

## Storage

SQLite (WAL) in the state directory:

- `events`: time, transport, protocol, source, destination, name used, attribution (bait, unknown, apex, ns, baseline, foreign), day, kind (always bait, kept for old rows), label for unknown names, how the address alone attributes it, summary, JSON detail, raw bytes.
- `certs`: one row per day with the issuance data above.

Enrichment: ASN and country from iptoasn's table, refreshed weekly.

## Results page

- Served by the binary on a local port, proxied by athena's nginx at `honeypot.hayl.in`.
- Daily names appear only as `[name YYYY-MM-DD]`, the zone as `[domain]`.
- Names scanners guessed show their real label, e.g. `www`.
- The main domain never appears anywhere on the page. Every displayed string is scrubbed, including paths, headers and SNI.
- Content: per day, cert in CT, first DNS query, first hit, totals; average lookups and hits per name by days since issuance; top ASNs, JA4s, user agents, paths and ports; baseline traffic.
- Request logs: per day (`/day/YYYY-MM-DD`), per guessed name (`/name/www`, `/name/@` for the bare domain) and per user agent substring (`/ua?q=`), each up to 2000 HTTP requests with headers and bodies, scrubbed.

## Deployment

- A NixOS container on athena, following `hosts/athena/tunnl.nix`: ephemeral, private network, v6 veth link, v4 handed to the host over v6, state bind-mounted from the host.
- The host routes 23.144.156.224/27 and 2602:fbf5:3:c7::/64 into the container.
- Inside the container: both prefixes on `lo`, the nftables TPROXY table plus `ip rule fwmark` and local route, the container firewall off, the binary as a DynamicUser with `CAP_NET_ADMIN` only.
- The name key is in sops. The state dir is persisted on the host.

## Later

- AbuseIPDB reporting: only clear malicious behaviour, one report per IP per day, hostnames never in comments, dry-run first.

## Status

- Live on athena since 2026-10-25 (launch day 20751): DNS, ACME (wildcard plus daily cert), TPROXY capture, results page at honeypot.hayl.in, container in `hosts/athena/honeypot.nix`, name key in `secrets/athena/honeypot.yaml`.
- Porkbun delegation and glue for `ns1`/`ns2.startguess.ing` are in place. To undo: nameservers back to `coco.bunny.net` and `kiki.bunny.net`, delete the glue, provider back to `bunny`.
- Not committed yet.
