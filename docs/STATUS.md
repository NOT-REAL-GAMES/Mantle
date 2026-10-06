# Mantle implementation status

Date: 2026-10-06. Execution environment: Windows x86_64, Rust 1.99.0.

## Implemented and exercised

- Rust WASM guest: exactly three `rustscript_v1` diagnostic imports, explicit ABI,
  capability and result failures, no generated JavaScript glue.
- Standalone native test bench: wasmi 2.0.0, no JavaScript or WASI, exact native
  call counters, import/type checks and bounded module/memory/table/fuel/stack limits.
- Native page authority: HTTPS origin normalization, separate profile/tab/page
  identity, top-level/foreground eligibility, one-use native activation, bounded
  ephemeral grants, revocation, replacement/close handling and buffer checks.
- Native callbacks use that authority. Their origin is artificial test-bench data;
  they do not prove browser identity extraction.
- Browser fixture, local server with optional TLS, and downloadable local evidence.
  Missing native imports fail visibly without a fallback.
- Pinned Servo checkout/build/run helper and portable check runner.

The Rust guest executed in the native test bench with result 42 and exactly one
native call per import. The test bench reports `browserPageRuntimeVerified:false`.
Portable test, formatting, native/WASM Clippy and WASM build results are recorded in
ignored artifacts by the check runner. Tests deliberately show that a JavaScript
mock can return 42 but cannot establish native browser evidence.

Latest portable run: 13 Rust tests and 3 Node tests passed; formatting and native/WASM
Clippy passed. The compiled guest was 1,006 bytes and consumed 1,533 fuel units in the
native bench. Browser fixture run in Chromium 154 on Windows passed WebGPU submission,
IndexedDB round trip, worker transfer and service-worker registration. The mint render
was visually present; physical display latency was not measured. Rustscript correctly
reported unavailable. These Chromium results are not Servo or Apple-device evidence.

## Not implemented or verified

| Required result | Status |
| --- | --- |
| Servo macOS browser build | Pending on MacBook |
| WALA running in Servo | Pending actual engine checks |
| Native imports inside Servo's page WASM runtime | Engine patch absent; Gate A unresolved |
| Engine-derived origin/profile identity | Pending realm integration; core tests alone insufficient |
| iOS/iPadOS engine port | Unproven |
| Worldwide iOS App Store eligibility | Blocked by current demonstrated route |
| ARKit, video/pose capture and WALA imports | Deferred until Gate A passes |
| Full browser shell, profiles UI, credentials and extensions | Deferred until Gate A passes |

No WALA source, authentication service or deployed bundle was changed for this
prototype. No Apple signing, build, device tracking, Servo compatibility or App
Store acceptance is claimed. See FEASIBILITY.md for actual source evidence.

## Next concrete MacBook task

Run the pinned engine against the compatibility fixture and WALA. Save real results
locally. Then implement the minimal three-import native engine binding with realm
identity and independently recorded call counters. Use the SDK and core authority;
do not pass page-provided origin strings or install JavaScript forwarding closures.

Stop and report a failed gate before extending the product. A regional entitlement,
simulator, separate WASM interpreter or working WebKit message bridge does not
satisfy the selected worldwide page-runtime requirement.
