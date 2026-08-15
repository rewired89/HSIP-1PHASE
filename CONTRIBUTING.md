# Contributing to HSIP

HSIP is a proprietary privacy product. Contributions are welcome — whether that is fixing a bug, improving documentation, adding a new tracker to the blocklist, or building a new integration.

By contributing you assign your rights to Dayana Sanchez (see License section below). Every contribution helps users trust the tool more.

---

## Before You Start

- Check [open issues](https://github.com/rewired89/HSIP-1PHASE/issues) to see if someone is already working on it.
- For significant changes, open an issue first to discuss approach before writing code.
- Small fixes (typos, doc improvements, minor bugs) — just open a PR.

---

## Development Setup

### Prerequisites

- Rust 1.87+ (`rustup update stable`)
- Node.js 20+ and npm (for the dashboard)
- Git

### Build

```bash
git clone https://github.com/rewired89/HSIP-1PHASE
cd HSIP-1PHASE

# Build the dashboard
cd dashboard && npm install && npm run build && cd ..

# Build the full binary with embedded dashboard
cargo build --release -p hsip-api --features hsip-api/embed-dashboard

# Run
./target/release/hsip-api
```

### Development mode (hot reload)

```bash
# Terminal 1 — API server
cargo run -p hsip-api

# Terminal 2 — dashboard with hot reload
cd dashboard && npm run dev
```

Dashboard dev server runs at `http://localhost:3001` and proxies `/v1/*` API calls to `http://localhost:7474` (desktop mode's default port — see `dashboard/vite.config.js` if you're running the API in server mode against `config.toml` instead, which defaults to port 3000).

### Run tests

```bash
cargo test --workspace
```

The full workspace test suite must pass before submitting a PR — don't rely on a hardcoded test count here, it grows with every feature and would only go stale; `cargo test --workspace` itself is the source of truth. First build compiles `hsip-verify`'s Z3 dependency from source (~8 min, one-time, cached afterward) — see `CLAUDE.md`'s "Including hsip-verify in the Build" if that surprises you.

---

## Project Structure

```
crates/            17 crates total — see CLAUDE.md's "Crate Map" for the full table
  hsip-api/        REST API server (main entry point) — core, do not break
  hsip-core/       Cryptographic primitives, no I/O — core, do not break
  hsip-dns/        DNS tracker blocker
  hsip-cli/        Command-line interface
  hsip-mcp/        MCP server (JSON-RPC over stdio) for AI clients
  hsip-verify/     Z3-backed formal verification of core security properties
  hsip-intercept/  Cross-platform network interception (Windows/Android/Linux/macOS)
  hsip-session/, hsip-net/, hsip-auth/, hsip-reputation/, hsip-gateway/,
  hsip-regenerative/, hsip-telemetry-guard/, hsip-integration-sdk/, hsip-common/
                   Supporting crates — stable, don't touch unless needed

dashboard/         React 18 frontend (embedded in release binary)
  src/pages/       20 UI pages, split across a Simple/Expert mode toggle
  src/data/        Tracker database (trackers.js)

browser-extension/ Chrome/Firefox extension
sdks/              Python, Node.js, Go SDKs
examples/          Integration examples
spec/              Formal protocol specification
docs/              Technical documentation
```

---

## What to Contribute

### Adding trackers to the blocklist

The tracker list lives in two places that must be kept in sync:
1. `dashboard/src/data/trackers.js` — frontend tracker database with descriptions
2. `browser-extension/rules.json` — declarativeNetRequest rules for the extension
3. `crates/hsip-dns/src/` — Rust DNS blocklist

To add a tracker:
1. Add an entry to `dashboard/src/data/trackers.js` with vendor, domain, description, category, and risk level
2. Add a rule to `browser-extension/rules.json` with the next available ID
3. Add the domain to the DNS blocklist in `crates/hsip-dns/src/`
4. Test that the tracker is blocked locally

### Adding integration examples

Add a directory under `examples/` with:
- A `README.md` explaining the use case
- Working code (Python, Node.js, Go, or Rust)
- Clear comments explaining what each step does

### Improving documentation

Documentation lives in `docs/`, `spec/`, and Markdown files at the root. Improvements to clarity, accuracy, or completeness are always welcome.

### Bug fixes

1. Add a test that reproduces the bug
2. Fix the bug
3. Verify the test passes
4. Submit a PR with both the test and the fix

### New features

For features that change the API or protocol, open an issue first to discuss the design. Protocol changes require updating `spec/` and `docs/PROTOCOL_SPEC.md`.

---

## Pull Request Guidelines

- Keep PRs focused — one concern per PR
- Include tests for new functionality
- Update documentation if your change affects user-facing behavior
- All tests must pass: `cargo test --workspace`
- Rust code should pass `cargo clippy --workspace` without warnings
- Dashboard code should pass `cd dashboard && npm run build` without errors

### Commit message format

Use clear, concise commit messages:

```
Add Hotjar to DNS blocklist

Hotjar is a session-recording tracker that captures mouse movements
and form inputs. Added to all three blocklist locations.
```

---

## Security Issues

**Do not open a public GitHub issue for security vulnerabilities.**

Email: **sanchezleal1989@gmail.com**

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Your suggested fix (if you have one)

We aim to respond within 48 hours and will credit you in the release notes.

---

## Code of Conduct

This project is built by people who believe in privacy as a fundamental right. Contributions are welcome from everyone. Be respectful. Focus on the work.

---

## License

By contributing to HSIP, you irrevocably assign all intellectual property rights in your contribution to Dayana Sanchez. Contributions may be used under proprietary commercial terms. See [LICENSE](LICENSE) for full details.
