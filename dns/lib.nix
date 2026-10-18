{ lib }:

rec {
  inherit lib;

  record = type: args: { inherit type args; };

  A =
    name: addr:
    record "A" [
      name
      addr
    ];
  AAAA =
    name: addr:
    record "AAAA" [
      name
      addr
    ];
  ALIAS =
    name: target:
    record "ALIAS" [
      name
      target
    ];
  CNAME =
    name: target:
    record "CNAME" [
      name
      target
    ];
  NS =
    name: target:
    record "NS" [
      name
      target
    ];
  PTR =
    name: target:
    record "PTR" [
      name
      target
    ];
  TXT =
    name: text:
    record "TXT" [
      name
      text
    ];
  MX =
    name: priority: target:
    record "MX" [
      name
      priority
      target
    ];
  SRV =
    name: priority: weight: port: target:
    record "SRV" [
      name
      priority
      weight
      port
      target
    ];
  IGNORE = name: record "IGNORE" [ name ];
  RDR =
    name: url:
    record "BUNNY_DNS_RDR" [
      name
      url
    ];

  migadu = domain: verify: [
    (MX "@" 10 "aspmx1.migadu.com.")
    (MX "@" 20 "aspmx2.migadu.com.")
    (TXT "@" "hosted-email-verify=${verify}")
    (TXT "@" "v=spf1 include:spf.migadu.com -all")
    (TXT "_dmarc" "v=DMARC1; p=none;")
    (CNAME "key1._domainkey" "key1.${domain}._domainkey.migadu.com.")
    (CNAME "key2._domainkey" "key2.${domain}._domainkey.migadu.com.")
    (CNAME "key3._domainkey" "key3.${domain}._domainkey.migadu.com.")
    (CNAME "autoconfig" "autoconfig.migadu.com.")
    (SRV "_autodiscover._tcp" 0 1 443 "autodiscover.migadu.com.")
    (SRV "_submissions._tcp" 0 1 465 "smtp.migadu.com.")
    (SRV "_imaps._tcp" 0 1 993 "imap.migadu.com.")
    (SRV "_pop3s._tcp" 0 1 995 "pop.migadu.com.")
  ];

}
