# Mantle: MacBook Codex continuation plan

## Assignment

Continue [NOT-REAL-GAMES/Mantle](https://github.com/NOT-REAL-GAMES/Mantle) on macOS.
Mantle is the browser; Rustscript is its native capability API. Read AGENTS.md,
README.md, DESIGN.md, FEASIBILITY.md and STATUS.md before editing.

Implement and validate stages 1–4 below, then investigate stage 5. This is an
execution handoff, not a request to restate the architecture. Deliver working
code, reproducible engine changes, measured results and an updated status report.
Continue independently within the selected design; stop the affected milestone
only for a concrete blocker or a product decision that cannot be inferred.

The immediate result is **a macOS Servo build running WALA and a Rust WASM guest
calling native functions in its page runtime with real origin/lifetime isolation**.
It is not yet a complete browser or an iOS App Store release.

## Current baseline and fixed decisions

The initial handoff baseline is commit
`daf40b98c3a7d1a62e0d2fc26f9c01b4fbd99e43`; use current main and inspect subsequent
changes rather than resetting to it. The portable Windows run passed 13 Rust
tests, 3 Node tests, formatting, native/WASM Clippy and the WASM build. Its real
Rust guest returned 42 with three native calls in **standalone wasmi**, not Servo.

Chromium passed the fixture's WebGPU clear, IndexedDB round trip, worker transfer
and service-worker registration. Rustscript was unavailable, as expected. None of
those results establishes Servo, Apple hardware, ARKit or App Store readiness.

Keep these decisions:

- Custom engine candidate: Servo revision in `engine/servo.json`, currently
  `964981f4c6eaa49a1f82700a4fb51afc76c43785`.
- Direct native calls from the **page's WASM runtime**; ordinary JS bootstrap is
  allowed, but JS forwarding/message transport is not the selected bridge.
- WALA stays a web application, with existing authentication, license, history,
  collaboration and recovery guards.
- macOS, iPhone and iPad are targets; worldwide iOS distribution remains required
  for shipping, but its unresolved route does not prevent macOS experiments.
- Pose recording and camera-video recording are separate toggles; either or both
  can operate while the user works in WALA.
- No engine rewrite, replacement engine, regional-only distribution or WebKit
  fallback without a new user decision.

## 1. Establish the Mac baseline

Clone into a new directory; preserve any existing checkout and local work:

```sh
git clone https://github.com/NOT-REAL-GAMES/Mantle.git
cd Mantle
git switch -c codex/macos-native-bridge
node scripts/check.mjs
```

If already cloned, inspect Git status and fetch before updating; do not reset or
clean. Use another branch name under `codex/` if the example already exists.

Record macOS version/architecture, Xcode version and selected developer directory,
Rustup/toolchain versions, Node version and available upstream prerequisites.
Use full Xcode and the pinned Servo README's prerequisites. Do not silently run
privileged installers, change the system toolchain or submit Apple agreements.
If a missing installation needs the human, state the exact package and command;
continue independent inspection while waiting.

Run from Mantle's root:

```sh
node scripts/servo.mjs checkout
node scripts/servo.mjs bootstrap
node scripts/servo.mjs build
```

Servo uses its upstream-pinned Rust 1.97.1; Mantle uses Rust 1.99.0. Keep both pins.
Do not run the checkout command over engine edits: it refuses a dirty checkout.

**Acceptance:** portable checks pass on macOS, the pinned engine builds, and its
actual binary opens an ordinary HTTPS page. Capture exact commands and failures.
An architecture, dependency or WebGPU failure is a reproducible blocker, not a
reason to replace the engine automatically.

## 2. Test the actual browser workload

In terminal A, leave the local fixture server running:

```sh
node scripts/serve.mjs
```

In terminal B:

```sh
node scripts/servo.mjs run http://127.0.0.1:8178/
node scripts/servo.mjs run https://not-real.games/wala/
```

Run the fixture and save its report under ignored `artifacts/`. Confirm the mint
render visually. HTTP loopback is sufficient for web-primitive checks only; the
native authority rejects HTTP and the direct-import proof requires trusted HTTPS.

Exercise WALA in Servo, using a disposable test project:

| Area | Required observation |
| --- | --- |
| Loading/rendering | WASM boots, WebGPU compositor works, resize and redraw work |
| Authentication | Real OAuth navigation/popup and cookies return to a valid session |
| Editing | Activated-license checks remain; paint/erase, undo/redo and rapid strokes work |
| Workers/storage | Brush worker starts, accepted input drains, IndexedDB save/reload succeeds |
| Input | Mouse navigation, keyboard, text focus and available pen/touch work without UI leakage |
| Files/recovery | Import/export works; network interruption retains recoverable local work |
| Collaboration | Two clients share edits and reject stale revisions without overwriting |

Request human sign-in or physical pen checks when actually needed; never fabricate
an activated session, paste credentials into logs, or bypass licensing. Mark any
unavailable hardware/auth check pending. Inspect the engine error and isolate a
small reproduction for failures before changing WALA.

Use a separate WALA checkout for necessary changes. No production deployment is
part of this assignment. Run WALA's own required tests/builds for affected code.

**Acceptance:** runtime evidence identifies which fixture and WALA checks passed,
failed or remain pending, with the exact engine revision and WALA bundle identity.
A GPU clear alone does not pass the WALA compositor check.

## 3. Prove the native page-WASM bridge

### Resolve the native-core toolchain boundary first

All current Mantle crates inherit `rust-version = "1.99"`. Linking
`rustscript-core` directly into Servo's 1.97.1 build will currently fail Cargo's
MSRV check even if its source is compatible.

Test the core and its locked dependencies under 1.97.1 before changing metadata:

```sh
cargo +1.97.1 test --locked -p rustscript-core --ignore-rust-version
cargo +1.97.1 clippy --locked -p rustscript-core --all-targets --ignore-rust-version -- -D warnings
```

If both pass, give **only rustscript-core** an explicit proved MSRV of 1.97.1 and
rerun its tests/Clippy without `--ignore-rust-version`, plus the normal1.99 check
runner. Retain the workspace and Servo pins. SDK/guest artifacts can remain 1.99
built because the interface is WASM, not Rust's native ABI.

If compatibility fails, capture the dependency or source error and stop that
integration step. Do not duplicate the permission broker, ignore MSRV in the
shipping build, or pass Rust objects between compiler versions through an
unverified native ABI.

### Implement the smallest engine experiment

Inspect the pinned Servo script runtime, realm construction, Navigator bindings,
native function generation and SpiderMonkey APIs. Prefer existing native binding
machinery over a custom interpreter or message protocol.

Implement exactly the existing `rustscript_v1` diagnostic contract:

| Import | WASM signature | Approved diagnostic result |
| --- | --- | --- |
| abi_version | () -> i32 | 1 |
| capabilities | () -> i32 | Diagnostic bit 1; no tracking/video bits |
| probe_add | (i32,i32) -> i32 | Wrapped i32 addition, 20 + 22 = 42 |

Use the fixture's proposed `navigator.rustscript` native import source. The page
passes its function references directly to `WebAssembly.instantiate`; do not
introduce executable JavaScript forwarding functions or `.bind` wrappers.
Ensure the native functions support that invocation without a JS receiver wrapper.

Capture caller identity from actual engine execution: profile, browsing context,
committed document/realm and trusted origin. Connect native lifecycle events to
the existing Broker, including same-origin replacements. A function's owning
realm is not sufficient if another realm can borrow its reference: reject a call
that would borrow another document's authority.

Wire native trusted activation and affirmative native approval for Diagnostic.
A page-supplied boolean or claimed trusted event is not permission. Use a narrow
native test permission surface; expose no camera or recording capability yet.
If asynchronous fixture checks outlive activation, move the native request to the
immediate user action or add a dedicated diagnostic button. Do not extend grant
validity to compensate for scheduling.

Use trusted HTTPS for this proof. Keep the certificate/key outside the repository,
with the certificate covering 127.0.0.1; never bypass certificate validation.

In terminal A:

```sh
MANTLE_TLS_CERT=/absolute/path/cert.pem MANTLE_TLS_KEY=/absolute/path/key.pem node scripts/serve.mjs
```

In terminal B:

```sh
node scripts/servo.mjs run https://127.0.0.1:8178/
```

Record native entry counters and a debugger call path for the actual compiled
guest. Hash the guest, engine binary and patch. A page result of 42 remains
`candidate`; independent engine evidence must show exactly one successful ABI,
capability and addition call with no JS forwarding and no separate WASM VM.

Test absent support, wrong ABI, missing capability, denied approval and revoked
authority. Mismatched ABI must short-circuit further capability/addition calls.
Normal web pages without Rustscript keep working. Respect CSP and normal WASM
validation; do not enable unsafe script execution globally.

### Preserve the engine patch in Mantle

`vendor/` is ignored. A working edit there alone is not a deliverable.

- Store the minimal ordered patch series in tracked `engine/patches/`, based on
  the exact engine pin. Include newly added engine files, manifest/lock changes
  and required native binding configuration; an unstaged Git diff omits new files.
- Add a small patch-application/check command only when the actual patch exists.
  Verify the upstream revision and refuse incompatible or locally modified files.
  Recognize already-applied patches without applying them twice.
- Confirm the series applies to a second clean pinned checkout and rebuilds.
  Preserve the working checkout; do not use recursive deletion to manufacture a test.
- Record patch hashes in the engine proof. The current build/run helper validates
  upstream HEAD but permits working-tree edits; HEAD alone does not identify the
  tested engine after patching.

**Acceptance:** the guest runs in Servo's actual page runtime, native provenance
is independently demonstrated, and another clean checkout can reproduce it.

## 4. Validate real origin and lifecycle isolation

Add meaningful engine/browser regression tests for the integration, reusing the
core tests rather than copying their assertions into another broker implementation.

Required cases:

- Two top-level origins cannot use each other's handles or native function references.
- Same-origin reload, redirects, invalid destinations, tab close, explicit revoke
  and backgrounding invalidate existing grants and pending native activations.
- Back/forward restoration and retained/detached realms do not resurrect authority.
- Frames and workers are denied in v1, including borrowed top-level functions.
- Regular/private profiles and separate tabs do not inherit permission implicitly.
- Denied native user approval cannot become a successful native call. Diagnostic
  has no OS permission; reserve OS-denial checks for the later ARKit capability.
- Pending callbacks, cancellation and GC/runtime destruction cannot dereference
  destroyed state; callbacks do not block rendering or retain WASM memory pointers.

Only test memory transfer once the bridge has such a call; scalar diagnostic
success does not prove future pointer/length safety. Follow core buffer checks
when extending the ABI. Keep unsupported tracking/recording bits clear.

**Acceptance:** real engine tests pass, no stale call increments successful native
counters, and context/lifetime provenance is demonstrated beyond artificial data.
Do not change the page fixture to self-certify engine provenance.

## 5. Assess iOS/iPadOS execution and distribution

Recheck current primary Apple documentation; FEASIBILITY.md is a dated assessment,
not permanent policy. Report these separately:

1. Servo's native dependencies and renderer/input surface on the target SDK.
2. WASM/SpiderMonkey execution under the actual permitted JIT/process arrangement.
3. Applicable signing, entitlements, lifecycle and physical-device execution.
4. Regional routes versus the selected worldwide App Store shipping target.

Start with the smallest applicable build/dependency experiment. Do not invent an
entitlement, disable platform protections, imply a simulator proves device support,
or turn a successful developer build into App Store approval. Submitting agreements,
entitlement requests or external communications requires human action/authorization.

If the worldwide route remains unavailable, mark **iOS shipping blocked** and keep
its concrete evidence. Useful macOS technical work can continue; do not silently
change to WKWebView, regional shipping or a different product architecture.

## 6. Subsequent work after the relevant gates pass

Follow DESIGN.md rather than scaffolding every later subsystem now:

1. Minimal macOS shell: clear origin/address, navigation, tabs and working OAuth.
2. Validated Apple platform layer: native session permissions, lifecycle and ARKit.
3. Typed pose capability: timestamps, rigid transforms, tracking quality and resets.
4. WALA adapter: detached navigation using `C0 * inverse(T0) * T(t)`, default one
   scene unit/metre, explicit scale/recenter, no acceleration integration or return springs.
5. Independent movement/video capture with native timestamps, local recoverable
   journal, native encoding and explicit WALA import retaining original recordings.
6. Phone-to-Mac delivery, then full-browser profiles, credentials, extensions and
   resource/security maintenance as separately validated milestones.

Tracking loss freezes the last valid camera and reports the limitation. Orientation
changes do not reset the world origin. Live movement remains local and outside
project history. MacBooks are not assumed to provide mobile ARKit world tracking.
Do not claim complete WebXR or general extension compatibility from the prototype.

## Deliverables and finish criteria

For each stage, update STATUS.md with passed/failed/pending checks and evidence.
Keep raw logs, captures, credentials, recordings and native artifacts ignored.
Commit concise reproducible findings and tests, not private sign-in data.

The first MacBook handoff is complete when it includes:

- A macOS portable-check run and pinned Servo build result.
- Actual fixture and WALA compatibility results with unresolved physical checks marked.
- A reproducible minimal native engine patch and independent call-path proof, or
  the exact source/build/runtime blocker preventing it.
- Origin/lifecycle regression results and a documented core/engine toolchain boundary.
- A separate iOS technical/distribution assessment and the next concrete task.

Run `node scripts/check.mjs` after portable changes, the pinned engine's relevant
build/tests after engine changes, and WALA's required suite if WALA is touched.
Do not repeat broad checks without a new change, failure or unresolved concern.

Use a `codex/` branch. Preserve unrelated work. A branch/PR should explain actual
behavior and validation, not claim later roadmap milestones are implemented.
When blocked, report the smallest reproducible blocker and preserve the patch,
test data and recovery needed to resume. Do not end with only a plan or offer to
continue when an authorized build, fix or validation can still be completed.
