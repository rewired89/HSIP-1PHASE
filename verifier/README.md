# HSIP Proof Check

A single static page that checks an HSIP proof or evidence pack in the browser. It runs the same open-source verifier as the `hsip-proof` command-line tool (`crates/hsip-proof`), compiled to WebAssembly by `crates/hsip-verify-web`. It makes no network requests after loading, and its security policy forbids contacting any other site.

## Use it

The page needs to be served over http (browsers will not run WebAssembly from a `file://` address):

```powershell
cd verifier
python3 -m http.server 8000    # use `python` on Windows; then open http://127.0.0.1:8000
```

Or upload the folder to any static host (GitHub Pages works). Anyone can host their own copy.

## Hosting

`.github/workflows/pages.yml` publishes this folder to GitHub Pages on every change. One-time setup: repository Settings, Pages, Source: GitHub Actions. The page then appears at `https://<owner>.github.io/<repo>/`.

## Rebuild the WebAssembly

```powershell
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.106 --locked   # must match the wasm-bindgen version in Cargo.lock
cargo build -p hsip-verify-web --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-dir verifier/pkg target/wasm32-unknown-unknown/release/hsip_verify_web.wasm
```

The page can also create a permission: fill in the rules, generate a key (made and kept in the page, downloaded as `my-key.json`), sign, and download `permission.json`. It is the same format `hsip-proof mandate-sign` produces and the console's Permissions page accepts.

A genuine result means the record is unaltered since it was signed. It does not mean the decision it describes was right. For a result you rely on in a dispute, run `hsip-proof verify` on your own machine.
