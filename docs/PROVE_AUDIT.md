# Auditing HSIP: free tools and where to send it

Status of the last self-scan (2026-10-18), all reproducible with the commands below:

- `cargo audit` found 12 known vulnerabilities in the lockfile. 11 were patch-level updates to transitive networking crates (`rustls`, `rustls-webpki`, `h2`, `bytes`, `time`) and are fixed in `Cargo.lock`. One remains: `rsa` (RUSTSEC-2023-0071, no patched release). It is pulled in by `sqlx-mysql`, which this project does not enable, and `cargo tree -p hsip-platform -i rsa` shows it is not built into the platform. Other warnings (unmaintained or unsound crates such as `anyhow`, `rand` 0.8, `event-listener`, `glib`) are informational and mostly come from older workspace crates.
- `npm audit` found a high-severity open-redirect XSS in `react-router-dom` 6.28.0. Fixed by pinning 6.30.6. One moderate advisory remains in `react-router` 6.x (open redirect via a backslash in `<Link>`/`useNavigate`, plus an SSR-only issue); its fix needs react-router 7, a breaking change. It is exploitable only when a link target is attacker-controlled; this console only navigates to fixed paths and server-issued ids, and has no SSR. Revisit when moving to v7. Dev-only findings affect build tooling (vite), not the shipped site.
- `cargo clippy -D warnings` and 30 tests pass; the PostgreSQL compatibility test passes.

This is a self-scan. It is not an independent audit.

## Free tools you can run today (PowerShell)

```powershell
# Dependencies with known vulnerabilities
cargo install cargo-audit --locked; cargo audit
cd console; npm audit --omit=dev; cd ..

# Licenses, banned or duplicate crates, advisories in one policy file
cargo install cargo-deny --locked; cargo deny init; cargo deny check

# Lints and unsafe usage
cargo clippy -p hsip-proof -p hsip-platform --all-targets -- -D warnings
cargo install cargo-geiger --locked; cargo geiger -p hsip-platform

# Secrets accidentally committed (scans full git history)
# https://github.com/gitleaks/gitleaks  (or trufflehog)
gitleaks detect --source . -v

# Vulnerable dependencies across Cargo and npm in one tool
# https://github.com/google/osv-scanner
osv-scanner -r .

# Static analysis rules for Rust and JS
# https://semgrep.dev  (open-source CLI is free)
semgrep scan --config auto crates/hsip-platform crates/hsip-proof console/src

# Dynamic scan of the running server (point it at http://127.0.0.1:8080)
# OWASP ZAP baseline scan, free: https://www.zaproxy.org
```

Deeper, free, and worth doing for the verifier and parser code:

- **Fuzzing** `hsip-proof` (it parses untrusted JSON): `cargo install cargo-fuzz`, then fuzz `verify_bundle` and `verify_pack` with arbitrary bytes. It must never panic.
- **Mutation testing**: `cargo install cargo-mutants`, `cargo mutants -p hsip-proof`. Shows which behaviors your tests do not actually pin.
- **Miri** (`cargo +nightly miri test -p hsip-proof`) for undefined behavior; the crate has no `unsafe`, so expect a clean run.

## Free services on GitHub

- **Dependabot** (alerts and update PRs) and **secret scanning**: Settings, Code security.
- **CodeQL code scanning** now supports Rust and JavaScript. GitHub documents it as free for public repositories; for private repositories it is part of paid GitHub Advanced Security. Confirm the terms for your repository's visibility before relying on it.
- **OpenSSF Scorecard** (`ossf/scorecard-action`) grades repository hygiene.

## Where to send it for a real, independent audit

Free is not the same as independent. Self-scans find known issues; they do not find design flaws.

1. **Before sending anyone the repo**, run the free tools above, fix what they find, and write down the threat model. `docs/PROVE_PLATFORM.md` "Honest limits" is a start. Auditors are expensive per hour; do not pay them to find what `cargo audit` finds.
2. **Scope it small**: `crates/hsip-proof` (format and verifier, about 700 lines) is the highest-value, cheapest thing to have reviewed. Then the platform's auth, tenant isolation and key handling (`auth.rs`, `crypto.rs`, `state.rs`, `routes/`).
3. **Community and low-cost options**: a Rust security community review (Rust Discord or users.rust-lang.org "security" threads), a university applied-cryptography group, or a bug-bounty style program once you have paying users.
4. **Funded open-source audits**: the Open Source Technology Improvement Fund (OSTIF) coordinates audits of open-source projects. It generally looks for projects with real adoption, so it fits the verifier later, not today. Check their current application process on their site.
5. **Paid firms** (needed once customers ask for a third-party report): Trail of Bits, NCC Group, Cure53, Least Authority, Zellic, Include Security, Doyensec. Ask for a fixed-scope quote on the verifier plus tenant isolation. Enterprise buyers will ask for this report.

## Recurring

CI (`.github/workflows/prove.yml`) runs `cargo audit` and `npm audit` on every push. Re-run the full list monthly and before each release.
