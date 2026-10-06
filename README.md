# Mantle

Public repository: [NOT-REAL-GAMES/Mantle](https://github.com/NOT-REAL-GAMES/Mantle).
Mantle is the browser product; Rustscript is its native capability API.

Custom browser research for macOS, iPhone and iPad. **Rustscript** is its proposed
page-WASM native capability API. This repository implements the portable part of
the first feasibility milestone, not a usable browser or an ARKit integration.

Read [the design](docs/DESIGN.md), [implementation status](docs/STATUS.md), and
[engine/distribution assessment](docs/FEASIBILITY.md) before continuing on a Mac.

## Run the implemented prototype

Install Rustup and Node.js (Node 22 or later). Rustup reads the checked-in toolchain
and installs Rust 1.99.0 plus the WASM target. There are no npm dependencies.

```sh
node scripts/check.mjs
node scripts/serve.mjs
```

Open http://127.0.0.1:8178/ and press **Run compatibility checks**. The page checks
WebGPU submission, IndexedDB transactions, transferable worker buffers, and service
worker registration. Rustscript should be **unavailable** in ordinary browsers.
These checks do not establish WALA compatibility or Servo support by themselves.

The Rust guest imports exactly `rustscript_v1.abi_version`, `capabilities`, and
`probe_add`. Its native test bench executes all three with no JavaScript runtime
and prints explicit scope information. The test bench uses an artificial HTTPS
page context; it is not a substitute for engine-derived origin isolation.

```sh
cargo run --locked -p native-probe -- target/wasm32-unknown-unknown/release/bridge_probe.wasm
```

The check runner stores local logs in ignored `artifacts/portable-checks.json` and
stops at the first failed command. Native artifacts and downloaded engines remain
outside source control.

## Continue on the MacBook

MacBook Codex should execute [the continuation plan](docs/MACBOOK_PLAN.md), starting
with the portable baseline, actual Servo/WALA checks and native page-runtime proof.

Clone the repository and verify the portable prototype first:

```sh
git clone https://github.com/NOT-REAL-GAMES/Mantle.git
cd Mantle
node scripts/check.mjs
```

Use a **separate** WALA checkout. Install Xcode and the upstream Servo prerequisites
described by the pinned upstream README. Do not change Servo's Rust toolchain to
match this workspace.

```sh
node scripts/servo.mjs checkout
node scripts/servo.mjs bootstrap
node scripts/servo.mjs build
```

In another terminal run `node scripts/serve.mjs`, then:

```sh
node scripts/servo.mjs run http://127.0.0.1:8178/
node scripts/servo.mjs run https://not-real.games/wala/
```

The supplied Servo commands evaluate the **unmodified** pinned engine. No native
import patch exists yet. `navigator.rustscript` in the fixture is a proposed engine
binding, not an existing Servo feature. The fixture passes function references
directly to WASM; it never supplies JavaScript forwarding implementations. A result
of 42 is only a **candidate** until independent native engine counters and call-path
evidence verify it. JavaScript can spoof the object, so the page never self-certifies.

Production native permissions require HTTPS. The HTTP loopback fixture tests web
primitives only; the core authority rejects HTTP. For direct-import tests, supply
a trusted certificate covering 127.0.0.1 and a matching key outside this repository:

```sh
MANTLE_TLS_CERT=/absolute/path/cert.pem MANTLE_TLS_KEY=/absolute/path/key.pem node scripts/serve.mjs
```

Do not bypass certificate verification. Camera/tracking/recording imports are not
implemented. Worldwide iOS App Store distribution with the selected custom-engine
requirement has no demonstrated supported route; regional exceptions do not pass
that gate. See FEASIBILITY.md for primary sources and explicit stop conditions.

## Repository contents

- `rustscript-sdk`: dependency-free WASM diagnostic imports and ABI errors.
- `bridge-probe`: compiled Rust guest with no wasm-bindgen or JS glue.
- `rustscript-core`: native page authority, ephemeral grants and buffer bounds.
- `native-probe`: bounded standalone WASM test bench using wasmi, not the page runtime.
- `web/`: compatibility page and isolated workers.
- `engine/servo.json`: fixed upstream revision; no moving-branch build claims.

For MacBook Codex: start with AGENTS.md and Gate A in DESIGN.md. Implement the engine
proof before adding a browser shell, native tracking, or recording. Preserve WALA
artwork, credentials, licensing and recovery. Never turn an unrun check into a pass.

## License

This repository is MIT licensed. Servo is downloaded separately and retains its
upstream license; dependencies retain their respective licenses.
