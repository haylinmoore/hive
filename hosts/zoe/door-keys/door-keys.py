import json
import os
import re
import secrets
import sys

path = sys.argv[1]
cmd = sys.argv[2] if len(sys.argv) > 2 else "list"
name = re.sub(r"[^A-Za-z0-9_-]", "", sys.argv[3]) if len(sys.argv) > 3 else ""


def load():
    try:
        with open(path) as f:
            data = json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return []
    return data if isinstance(data, list) else []


def save(keys):
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(keys, f)
    os.chmod(tmp, 0o640)
    os.replace(tmp, path)


keys = load()

if cmd == "list":
    print(json.dumps({"keys": keys}))
elif cmd in ("add", "remove") and name:
    keys = [k for k in keys if k.split(":")[0] != name]
    if cmd == "add":
        keys.append(f"{name}:{secrets.token_hex(16)}")
    save(keys)
