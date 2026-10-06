# Mantle

Start with Gate A in docs/DESIGN.md. This is a feasibility prototype, not a shipping browser.
WALA and its authentication repository are integration references; preserve unrelated edits.
Use the pinned Rust toolchain. Servo uses its own upstream-pinned toolchain in vendor/servo.
Run cargo test --workspace, cargo fmt --all --check, cargo clippy --workspace --all-targets -- -D warnings,
and node --test tests/*.test.mjs after portable changes. Build the WASM probe and run the native probe.
Keep native builds, recordings, credentials and engine checkout out of Git.
Never report a standalone WASM runtime as the page runtime. Never substitute a JavaScript native-call adapter.
Track Apple build, browser-runtime and physical-device checks separately; unsupported checks remain pending.
Worldwide iOS distribution is a required shipping gate, not an assumption.
