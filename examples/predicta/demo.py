#!/usr/bin/env python3
"""
Demo: Predicta's trades recorded on HSIP, checked against a signed permission.

Needs a running HSIP platform and the hsip-proof tool:
    cargo run -p hsip-platform                      # http://127.0.0.1:8080
    cargo build -p hsip-proof                       # target/debug/hsip-proof
    python3 examples/predicta/demo.py

It signs up a throwaway organization, records six Predicta-shaped trades
through the real connector (examples/predicta/hsip_client.py), signs a
permission with a fresh key, and shows which trades fall outside it. Then it
re-runs both checks offline with the open tool. Output files go to ./demo-out.
"""
import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
ap = argparse.ArgumentParser()
ap.add_argument("--server", default="http://127.0.0.1:8080")
ap.add_argument("--hsip-proof", default=os.path.join(HERE, "..", "..", "target", "debug", "hsip-proof"))
ap.add_argument("--out", default="demo-out")
args = ap.parse_args()
os.makedirs(args.out, exist_ok=True)


def say(msg):
    print(f"\n== {msg}")


def api(method, path, body=None, token=None):
    req = urllib.request.Request(args.server + path, method=method, data=None if body is None else json.dumps(body).encode(),
                                 headers={"Content-Type": "application/json", **({"Authorization": f"Bearer {token}"} if token else {})})
    with urllib.request.urlopen(req, timeout=20) as r:
        raw = r.read()
        return json.loads(raw) if raw else None


def tool(*a):
    p = subprocess.run([args.hsip_proof, *a], capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


say("1. A throwaway HSIP organization, one stream, one ingest key")
email = f"demo-{uuid.uuid4().hex[:8]}@example.com"
su = api("POST", "/v1/auth/signup", {"email": email, "password": "correct-horse-battery", "name": "Demo", "org_name": "Predicta demo"})
tok, org = su["token"], su["org_id"]
sid = api("POST", f"/v1/orgs/{org}/streams", {"name": "Predicta trades"}, tok)["id"]
key = api("POST", f"/v1/orgs/{org}/api-keys", {"name": "predicta", "scope": "ingest", "stream_id": sid}, tok)["token"]

os.environ.update(HSIP_API_URL=args.server, HSIP_API_KEY=key, HSIP_STREAM=sid, HSIP_DISCLOSE_NOTIONAL="true")
sys.path.insert(0, HERE)
import hsip_client  # noqa: E402  the real connector, same call as Predicta's code

say("2. Predicta places trades; each one is recorded (only a hash and small labels leave Predicta)")
trades = [
    ("buy", "high_value_intraday", {"symbol": "AAPL", "qty": 10, "entry_price": 190.0}),
    ("sell", "high_value_intraday", {"symbol": "AAPL", "qty": 10, "exit_price": 193.5}),
    ("buy", "low_value_swing", {"symbol": "MSFT", "qty": 4, "entry_price": 420.0}),
    ("buy", "high_value_intraday", {"symbol": "NVDA", "qty": 5, "entry_price": 900.0}),  # $4,500: over the size limit
    ("sell", "high_value_intraday", {"symbol": "NVDA", "qty": 5, "exit_price": 905.0}),
    ("buy", "low_value_swing", {"symbol": "AMD", "qty": 8, "entry_price": 150.0}),        # 6th in the hour: over the rate limit
]
for side, strat, p in trades:
    p = {**p, "side": side, "predicta_trade_id": uuid.uuid4().hex[:8], "alpaca_order_id": uuid.uuid4().hex}
    r = hsip_client.attest_transaction(side, strat, p, model_version="v4")
    print(f"   {side:4} {p['symbol']:5} -> proof #{r['seq'] if r else 'FAILED'}")

say("3. You sign a permission with your own key (it never leaves this machine)")
kc, kout, _ = tool("keygen")
keyfile = os.path.join(args.out, "my-key.json")
open(keyfile, "w").write(kout)
now = int(time.time() * 1000)
body = {"format": "hsip-mandate/1", "mandate_id": "predicta-limits", "grantor_key": "", "agent": "predicta",
        "valid_from_ms": now - 3600_000, "valid_until_ms": now + 30 * 86400_000,
        "rules": {"allowed_actions": ["buy", "sell"], "max_amount": {"field": "notional_usd", "limit": 3000},
                  "max_per_period": {"count": 5, "period_secs": 3600}}}
bodyfile = os.path.join(args.out, "rules.json")
json.dump(body, open(bodyfile, "w"))
rc, mandate, err = tool("mandate-sign", keyfile, bodyfile)
assert rc == 0, err
permfile = os.path.join(args.out, "permission.json")
open(permfile, "w").write(mandate)
print("   Rules: buy or sell only, at most $3,000 per trade, at most 5 trades per hour")
api("POST", f"/v1/orgs/{org}/mandates", json.loads(mandate), tok)

say("4. The platform checks every recorded trade against the permission")
rep = api("GET", f"/v1/orgs/{org}/mandates/predicta-limits/report?stream_id={sid}", token=tok)["report"]
print(f"   {rep['checked']} trades checked, {len(rep['violations'])} outside the permission:")
for v in rep["violations"]:
    print(f"   - trade #{v['seq']}: {v['rule']}: {v['detail']}")

say("5. Anyone can repeat this offline with the open tool: evidence pack + permission")
ev = api("POST", f"/v1/orgs/{org}/evidence", {"stream_id": sid, "title": "Predicta demo"}, tok)
packfile = os.path.join(args.out, "evidence-pack.json")
req = urllib.request.Request(args.server + f"/v1/orgs/{org}/evidence/{ev['id']}/pack", headers={"Authorization": f"Bearer {tok}"})
open(packfile, "wb").write(urllib.request.urlopen(req, timeout=20).read())
rc, out, _ = tool("verify", packfile)
print("   hsip-proof verify      ->", out.strip().splitlines()[-1])
rc, out, _ = tool("mandate-check", permfile, packfile)
print("   hsip-proof mandate-check ->", out.strip().splitlines()[-1])

print(f"\nFiles in {args.out}/: permission.json, evidence-pack.json. Drop both on the Proof Check page to see the same result in a browser, offline.")
print(f"Console: {args.server}  (sign in as {email} / correct-horse-battery, open Permissions and Evidence)")
