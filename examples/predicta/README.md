# Predicta + HSIP

`hsip_client.py` is a drop-in replacement for Predicta's `fetchers/hsip_client.py`. It keeps the same `attest_transaction(decision_type, strategy_id, payload, model_version)` call, so Predicta's call sites do not change, but it talks to the current HSIP platform (`POST /v1/ingest`) instead of the retired Phase 1 server.

Set these in Predicta's environment:

```
HSIP_API_URL=http://127.0.0.1:8080          # or your hosted HSIP platform
HSIP_API_KEY=hsp_...                        # an ingest key from the HSIP console, API keys page
HSIP_STREAM=<stream id or slug>             # only if the key is not bound to a stream
HSIP_DISCLOSE_NOTIONAL=true                 # optional, see below
```

Only a SHA-256 fingerprint of the trade and a few labels (model version, strategy) leave Predicta. With `HSIP_DISCLOSE_NOTIONAL=true` it also records a rounded dollar amount, which lets a permission cap trade size. That reveals trade size to the platform, so it is off by default.

## Run the demo

```powershell
cargo run -p hsip-platform            # terminal 1
cargo build -p hsip-proof
python3 examples/predicta/demo.py     # terminal 2
```

It records six Predicta-shaped trades through the real connector, signs a permission (buy or sell only, at most $3,000 a trade, at most 5 trades an hour), shows the three trades that fall outside it, and repeats both checks offline with `hsip-proof`. Drop `demo-out/permission.json` and `demo-out/evidence-pack.json` on the Proof Check page (`verifier/`) to see the same result in a browser.

Nothing here changes Predicta. Copy `hsip_client.py` over Predicta's `fetchers/hsip_client.py` when you want it live.
