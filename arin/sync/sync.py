import ipaddress
import json
import os
import re
import socket
import sys
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET

import dns.dnssec
import dns.exception
import dns.message
import dns.query
import dns.rdatatype

API = "https://reg.arin.net/rest"
CORE = "http://www.arin.net/regrws/core/v1"
RPKI = "http://www.arin.net/regrws/rpki/v1"

ET.register_namespace("", CORE)
for prefix, uri in [("messages", "messages"), ("ticket", "shared-ticket"), ("ttl", "ttl"), ("rpki", "rpki")]:
    ET.register_namespace(prefix, f"http://www.arin.net/regrws/{uri}/v1")

RED, GREEN, YELLOW, RESET = "\033[31m", "\033[32m", "\033[33m", "\033[0m"


def request(method, path, body=None, ctype="application/xml"):
    req = urllib.request.Request(API + path, data=body.encode() if body else None, method=method)
    req.add_header("Authorization", "ApiKey " + os.environ["ARIN_API_KEY"])
    req.add_header("Accept", "application/xml")
    if body:
        req.add_header("Content-Type", ctype)
    try:
        with urllib.request.urlopen(req) as r:
            return r.read()
    except urllib.error.HTTPError as e:
        sys.exit(f"{method} {path}: {e.code}\n{e.read().decode()}")


def local(el):
    return el.tag.rsplit("}", 1)[-1]


def find(el, name):
    return [e for e in el.iter() if local(e) == name]


def text(el, name):
    found = find(el, name)
    return found[0].text if found else None


def net(prefix):
    return ipaddress.ip_network(prefix)


def address(s):
    if ":" in s:
        return str(ipaddress.ip_address(s))
    return ".".join(str(int(o)) for o in s.split("."))


def reverse_zones(block):
    n = net(block)
    if n.version == 4:
        subs = n.subnets(new_prefix=24) if n.prefixlen < 24 else [n]
        return [".".join(reversed(str(s.network_address).split(".")[:3])) + ".in-addr.arpa" for s in subs]
    nibbles = -(-n.prefixlen // 4)
    subs = n.subnets(new_prefix=nibbles * 4) if nibbles * 4 != n.prefixlen else [n]
    return [".".join(reversed(s.network_address.exploded.replace(":", "")[:nibbles])) + ".ip6.arpa" for s in subs]


def live_ds(zone, nameserver):
    ip = socket.getaddrinfo(nameserver, 53, proto=socket.IPPROTO_UDP)[0][4][0]
    answer = dns.query.udp(dns.message.make_query(zone, dns.rdatatype.DNSKEY), ip, timeout=5)
    keys = [rr for rrset in answer.answer for rr in rrset if rr.flags & 1]
    if not keys:
        raise LookupError(f"{nameserver} serves no DNSKEY for {zone}, push its dns zone first")
    return {ds_string(dns.dnssec.make_ds(zone + ".", key, "SHA256")) for key in keys}


def ds_string(ds):
    return f"{ds.key_tag} {ds.algorithm} {ds.digest_type} {ds.digest.hex().upper()}"


class Plan:
    def __init__(self):
        self.lines = []
        self.errors = []
        self.actions = []

    def add(self, kind, what, action=None):
        color = {"+": GREEN, "-": RED, "±": YELLOW}[kind]
        label = {"+": "CREATE", "-": "DELETE", "±": "MODIFY"}[kind]
        self.lines.append(f"{color}{kind} {label} {what}{RESET}")
        if action:
            self.actions.append(action)

    def error(self, message):
        self.errors.append(message)


def check_blocks(state, plan):
    for block, b in state["blocks"].items():
        for key, version in [("route", 4), ("route6", 6)]:
            for r in b[key]:
                if net(r["prefix"]).version != version:
                    plan.error(f"{block}: {r['prefix']} belongs in {'route6' if version == 4 else 'route'}")
        for r in b["route"] + b["route6"] + b["rpki"]:
            if not net(r["prefix"]).subnet_of(net(block)):
                plan.error(f"{block}: {r['prefix']} is outside the block")
        for zone in b["rdns"]:
            if zone not in reverse_zones(block):
                plan.error(f"{block}: {zone} is not one of its reverse zones")


def plan_roas(state, plan):
    org = state["org"]
    current = ET.fromstring(request("GET", f"/roa/{org}"))
    have = {}
    for spec in find(current, "roaSpec"):
        resources = frozenset(
            (f"{address(text(r, 'startAddress'))}/{text(r, 'cidrLength')}", int(text(r, "maxLength")))
            for r in find(spec, "resources")
        )
        have[(text(spec, "name") or "", int(text(spec, "asNumber")), resources)] = text(spec, "roaHandle")

    want = set()
    for block, b in state["blocks"].items():
        for origin in sorted({r["origin"] for r in b["rpki"]}):
            resources = frozenset(
                (str(net(r["prefix"])), r.get("maxLength", net(r["prefix"]).prefixlen))
                for r in b["rpki"]
                if r["origin"] == origin
            )
            want.add(("hive " + re.sub(r"[^A-Za-z0-9_ -]", "-", block), origin, resources))

    def describe(key):
        name, origin, resources = key
        return f"ROA {name or '(unnamed)'} (AS{origin}): " + ", ".join(f"{p} max /{m}" for p, m in sorted(resources))

    deletes = [have[k] for k in have if k not in want]
    adds = [k for k in want if k not in have]
    for k in have:
        if k not in want:
            plan.add("-", describe(k))
    for k in adds:
        plan.add("+", describe(k))
    if deletes or adds:
        plan.actions.append(lambda: roa_transaction(org, deletes, adds))


def roa_transaction(org, deletes, adds):
    root = ET.Element("rpkiTransaction", xmlns=RPKI)
    if deletes:
        d = ET.SubElement(root, "roaSpecDelete")
        for handle in deletes:
            ET.SubElement(d, "roaHandle", autoLink="false").text = handle
    if adds:
        a = ET.SubElement(root, "roaSpecAdd")
        for name, origin, resources in adds:
            spec = ET.SubElement(a, "roaSpec")
            ET.SubElement(spec, "autoLink").text = "false"
            ET.SubElement(spec, "asNumber").text = str(origin)
            ET.SubElement(spec, "name").text = name
            res = ET.SubElement(spec, "resources")
            for prefix, max_length in sorted(resources):
                r = ET.SubElement(res, "roaSpecResource")
                ET.SubElement(r, "startAddress").text = str(net(prefix).network_address)
                ET.SubElement(r, "cidrLength").text = str(net(prefix).prefixlen)
                ET.SubElement(r, "maxLength").text = str(max_length)
    request("POST", f"/rpki/{org}", ET.tostring(root, encoding="unicode"))


def plan_routes(state, plan):
    org = state["org"]
    current = ET.fromstring(request("GET", f"/org/{org}/routes"))
    have = {(str(net(text(r, "prefix"))), text(r, "originAS").upper()) for r in find(current, "routeRef")}
    want = {
        (str(net(r["prefix"])), f"AS{r['origin']}")
        for b in state["blocks"].values()
        for r in b["route"] + b["route6"]
    }
    for prefix, origin in sorted(have - want):
        plan.add("-", f"route {prefix} {origin}", lambda p=prefix, o=origin: request("DELETE", f"/irr/route/{p}/{o}"))
    for prefix, origin in sorted(want - have):
        plan.add("+", f"route {prefix} {origin}", lambda p=prefix, o=origin: create_route(state, p, o))


def create_route(state, prefix, origin):
    kind = "route6" if net(prefix).version == 6 else "route"
    rpsl = "\n".join([
        f"{kind}: {prefix}",
        f"origin: {origin}",
        "descr: managed by hive",
        f"admin-c: {state['poc']}",
        f"tech-c: {state['poc']}",
        f"mnt-by: MNT-{state['org']}",
        "source: ARIN",
    ])
    request("POST", f"/irr/route/{prefix}/{origin}", rpsl + "\n", "application/rpsl")


def plan_delegations(state, plan):
    for block, b in state["blocks"].items():
        for zone in reverse_zones(block):
            current = ET.fromstring(request("GET", f"/delegation/{zone}"))
            have_ns = sorted(n.text.lower().rstrip(".") for n in find(current, "nameserver"))
            have_ds = {
                f"{text(k, 'keyTag')} {text(k, 'algorithm')} {text(k, 'digestType')} {text(k, 'digest').upper()}"
                for k in find(current, "delegationKey")
            }
            wanted = b["rdns"].get(zone, {"nameservers": [], "ds": []})
            want_ns = sorted(n.lower().rstrip(".") for n in wanted["nameservers"])
            try:
                want_ds = live_ds(zone, wanted["nameservers"][0]) if wanted["ds"] == "auto" else set(wanted["ds"])
            except (LookupError, OSError, dns.exception.DNSException) as e:
                plan.error(f"{zone}: {e}")
                continue
            if have_ns == want_ns and have_ds == want_ds:
                continue
            changes = []
            if have_ns != want_ns:
                changes.append(f"NS {', '.join(have_ns) or 'none'} -> {', '.join(want_ns) or 'none'}")
            if have_ds != want_ds:
                changes.append(f"DS {', '.join(sorted(have_ds)) or 'none'} -> {', '.join(sorted(want_ds)) or 'none'}")
            plan.add("±", f"rDNS {zone}: " + "; ".join(changes), lambda z=zone, c=current, n=want_ns, d=want_ds: put_delegation(z, c, n, d))


def put_delegation(zone, current, nameservers, ds):
    children = list(current)
    for child in children:
        if local(child) in ("delegationKeys", "nameservers"):
            current.remove(child)
    keys = ET.Element(f"{{{CORE}}}delegationKeys")
    for record in sorted(ds):
        tag, algorithm, digest_type, digest = record.split()
        key = ET.SubElement(keys, f"{{{CORE}}}delegationKey")
        ET.SubElement(key, f"{{{CORE}}}algorithm").text = algorithm
        ET.SubElement(key, f"{{{CORE}}}digest").text = digest
        ET.SubElement(key, f"{{{CORE}}}digestType").text = digest_type
        ET.SubElement(key, f"{{{CORE}}}keyTag").text = tag
    servers = ET.Element(f"{{{CORE}}}nameservers")
    for name in nameservers:
        ET.SubElement(servers, f"{{{CORE}}}nameserver").text = name
    current.insert(0, keys)
    current.append(servers)
    request("PUT", f"/delegation/{zone}", ET.tostring(current, encoding="unicode"))


def plan_sets(state, plan):
    org = state["org"]
    current = ET.fromstring(request("GET", f"/org/{org}/as-sets"))
    have = [ref.get("name") for ref in find(current, "asSetRef")]
    for name in have:
        if name not in state["sets"]:
            plan.add("-", f"as-set {name}", lambda n=name: request("DELETE", f"/irr/as-set/{n}"))
            continue
        doc = ET.fromstring(request("GET", f"/irr/as-set/{name}"))
        members = sorted(m.get("name") for m in find(doc, "member"))
        want = sorted(state["sets"][name])
        if members != want:
            plan.add("±", f"as-set {name}: {', '.join(members)} -> {', '.join(want)}", lambda n=name, d=doc, w=want: put_set(n, d, w))
    for name in state["sets"]:
        if name not in have:
            plan.add("+", f"as-set {name}: {', '.join(state['sets'][name])}", lambda n=name: create_set(state, n))


def put_set(name, doc, members):
    for el in find(doc, "members"):
        for m in list(el):
            el.remove(m)
        for member in members:
            ET.SubElement(el, f"{{{CORE}}}member", name=member)
    request("PUT", f"/irr/as-set/{name}", ET.tostring(doc, encoding="unicode"))


def create_set(state, name):
    lines = [f"as-set: {name}", "descr: managed by hive"]
    lines += [f"members: {m}" for m in state["sets"][name]]
    lines += [f"admin-c: {state['poc']}", f"tech-c: {state['poc']}", f"mnt-by: MNT-{state['org']}", "source: ARIN"]
    rpsl = "\n".join(lines)
    request("POST", f"/irr/as-set?orgHandle={state['org']}", rpsl + "\n", "application/rpsl")


def main():
    if len(sys.argv) != 3 or sys.argv[1] not in ("preview", "push"):
        sys.exit("usage: arin-sync {preview|push} STATE.json")
    with open(sys.argv[2]) as f:
        state = json.load(f)

    plan = Plan()
    check_blocks(state, plan)
    if not plan.errors:
        plan_roas(state, plan)
        plan_routes(state, plan)
        plan_delegations(state, plan)
        plan_sets(state, plan)

    for line in plan.lines:
        print(line)
    for e in plan.errors:
        print(f"{RED}! {e}{RESET}")
    print(f"{len(plan.lines)} changes, {len(plan.errors)} errors")

    if plan.errors:
        sys.exit(1)
    if sys.argv[1] == "push":
        for action in plan.actions:
            action()
        print("done")


if __name__ == "__main__":
    main()
