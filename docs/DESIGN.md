# Mantle: Rustscript browser design and implementation handoff

Approved design, named Mantle by the user on 2026-10-06. This document specifies the
product; STATUS.md distinguishes implemented code from future work.

## 1. Purpose and fixed requirements

Build a full browser for macOS, iPhone and iPad that supports normal websites and
lets Rust compiled to WebAssembly call permitted native capabilities directly from
the page's WASM runtime. **Rustscript** initially names an SDK and capability ABI,
not a new source language. JavaScript remains available to normal web applications.

The first application is WALA virtual production: native ARKit position/orientation
tracking, plus independently selectable camera-movement and camera-video recording.
WALA remains a web editor. Live tracking must not rewrite authored cameras or project
history. Existing authentication, activated-license rules, collaboration and recovery
remain authoritative.

The user selected a custom engine, a genuinely JavaScript-free native-call path, and
worldwide iOS App Store distribution. Do not silently replace these with WKWebView
script messages, a companion WASM interpreter, regional-only shipping or a native
WALA rewrite. Worldwide distribution is currently an unresolved shipping blocker;
see the primary-source assessment in FEASIBILITY.md.

## 2. Architecture and Gate A

Use Servo as the first engine candidate, pinned in engine/servo.json. Reuse its
rendering, JavaScript, WASM and web platform; do not create a new engine or interchangeable
backends before a real second implementation exists. Keep the engine patch minimal.

Separate native browser policy from engine integration and thin Apple platform code:

| Component | Responsibility |
| --- | --- |
| Rust browser core | Navigation state, profiles, capability permissions and recording metadata |
| Servo integration | Page realm identity, native imports, web rendering and runtime lifecycle |
| Apple platform layer | Native UI, ARKit, camera encoder, Keychain, system permissions and lifecycle |

Use Swift/Apple frameworks where they directly provide the required service; Rust does
not remove Xcode, Apple signing or native framework requirements. The browser repository,
WALA and its shared authentication service stay separate.

Before full-browser work, prove all six Gate A results:

1. On macOS, pinned Servo loads an ordinary HTTPS page and executes JS, WASM and WebGPU.
2. Current WALA authenticates, draws, runs workers, stores/reloads and recovers in Servo.
3. A Rust WASM guest in that page runtime calls native functions without JS forwarding.
4. Real origins, frames, profiles, navigation and revocation cannot reuse another page's authority.
5. An applicable supported iOS engine/signing/entitlement route can run on physical hardware.
6. Worldwide App Store eligibility has a substantiated route, separately from developer builds.

The supplied portable prototype does not pass these browser/device gates. Stop the
affected milestone when a requirement fails. Report concrete evidence instead of
starting an engine rewrite or quietly weakening a requirement.

## 3. Rustscript execution, ABI and trust boundary

Native imports use a versioned namespace, initially `rustscript_v1`. The supplied guest
tests three scalar diagnostic calls: `abi_version`, `capabilities`, and `probe_add`.
JavaScript may instantiate the module with engine-native function references; it may
not implement forwarding closures in the native-call path. A JS function object with
a native engine entry point differs from executable JavaScript forwarding code.

The proposed browser exposure for the proof is `navigator.rustscript`. Bind it within
the relevant engine realm, retain safe native ownership, and deny unsupported frame/
worker requests. This binding is not an existing Servo API. A webpage can spoof it;
passing the page fixture cannot independently establish native provenance.

The engine must capture the caller's trusted origin, profile, browsing context and
document/runtime identity. Never accept the page's own origin string, token or IPC
claims as authority. Direct native counters and debugger call-path evidence must
establish the bridge. Engine hooks must respect CSP and existing WASM validation.

After proof, add only the required tracking and recording capabilities:

- Discover available services and request permission.
- Start/stop/read tracking and receive quality/reset status.
- Start/stop movement or video recording independently.
- Retrieve/export completed recordings after explicit user action.

Use integer handles and validated caller-owned WASM buffers. Validate pointer/length
arithmetic on every call, copy into WASM memory only while its storage is safely
accessible, and never retain a raw pointer across memory growth, GC, reentry or async
completion. Handle version mismatches and expired handles explicitly. Async native
work must not block the script/rendering thread.

Do not expose arbitrary FFI, native pointers, ambient filesystem access, unrestricted
execution or downloaded native libraries. Use a latest-pose buffer for viewing and a
separate ordered timestamp stream for recording. Report gaps and backpressure.

Require HTTPS, a foreground top-level page, native user activation and affirmative
permission UI. Distinguish tracking, movement recording, video recording and export.
Tracking consent does not grant recording. Navigate/reload/close/revoke/background
invalidates active authority. Every native callback revalidates lifetime before
delivery. Permissions are profile-specific and private grants are ephemeral.

The current core has a 64-page/64-grant bounded prototype budget, a 30-second one-use
native activation window, and no durable grants. Origin parsing uses the URL library.
These are tested native policy primitives, not a complete permission UI or process sandbox.
Future persistence must be designed before adding remembered permissions.

## 4. Native world tracking and WALA camera behavior

Use ARKit world tracking on supported iPhone/iPad hardware, checking device capability.
Basic tracking does not require LiDAR. A MacBook initially consumes recorded motion or
a paired phone; do not assume it has the same mobile world-tracking provider.
[Apple ARWorldTrackingConfiguration](https://developer.apple.com/documentation/arkit/arworldtrackingconfiguration).

Provider frames contain a session/sequence ID, monotonic timestamp, camera-to-world
rigid transform, tracking state/limitation, intrinsics/source dimensions when present,
display orientation and reset events. Document one right-handed metre-based convention
and convert at the provider boundary exactly once.

At activation capture native pose T0 and WALA detached camera C0. Apply subsequent
poses as `C(t) = C0 * inverse(T0) * T(t)` after coordinate conversion. Default scale is
one scene unit per metre, with an explicit scale setting for differently authored scenes.
Translations remain in the tracked world frame when the phone turns. Do not double-
integrate acceleration, add return springs, continuously recapture offsets or run legacy
accelerometer locomotion at the same time.

Freeze the last valid view on tracking loss and display the limitation. Resets and
relocalization must not cause an unexplained jump; require recentering where continuity
cannot be established. Orientation changes align image/viewport rendering without
resetting the world origin.

WALA feature-detects Rustscript through a small capability adapter and reuses its
existing full detached-camera transform. Live navigation stays outside artwork/history.
Authored-camera animation is created only by explicit recording/import. UI-owned
touches cannot pan the camera. Normal web browsers, pen drawing, touch navigation,
licensing and collaboration retain existing behavior.

Rustscript tracking is not a complete WebXR claim. A later WebXR implementation must
meet session, reference-space, reset, permission and rendering contracts. Camera access
is a separate capability; a partial navigator.xr shim is not full compatibility.
[WebXR](https://www.w3.org/TR/webxr/),
[Raw Camera Access](https://immersive-web.github.io/raw-camera-access/).

## 5. Recording and simultaneous WALA use

Provide independent **Record movement** and **Record video** toggles while live
navigation continues. Microphone capture is outside the initial scope. Show duration,
storage, tracking quality, a persistent recording indicator and a stop action.

Use native capture timestamps as the authoritative clock. Establish a session time
origin and preserve mapping to encoded presentation timestamps. For combined capture
retain frame-associated poses, intrinsics, dimensions, dropped frames, missing poses,
orientation changes and resets. Arrival time, wall-clock JS timestamps and assumed
constant frame rates are not synchronization mechanisms.

Encode through Apple's native facilities, defaulting to a supported H.264 configuration
up to 1080p/30fps. Record the actual selected configuration. Do not use per-frame JSON
or Base64. Camera passthrough is optional and requires a measured native texture/image
path; obtaining navigation poses does not require copying video through the page.

Write locally before export/upload. Use an append-only metadata journal beside the
video so interrupted captures retain completed data. Finalized bundles contain a
versioned manifest, optional video, optional timestamped translation/quaternion poses,
coordinate/timing metadata and discontinuities. Preserve provider precision and originals.
Do not report recording success until finalization succeeds.

On storage or encoder failure, stop safely and keep completed data. WALA receives live
poses immediately; finalized media/movement enter through an explicit import into a
staged draft using existing edit guards and quotas. Rejected authorization, transport,
quota or project commits never delete originals. No automatic cloud upload.

Add phone-to-Mac delivery only after local recording/tracking works: explicit pairing,
short-lived token and authenticated encryption; no automatic trust of LAN senders.
View delivery stays bounded; recording uses phone capture time, not network arrival.
Remote poses/status precede live video, whose bandwidth and timing require measurement.

## 6. Full-browser stages

After Gate A, add the first usable shell: clear address/origin display, search, tabs,
history navigation, reload/stop, session restore, OAuth/new-window handling, downloads,
file uploads, sharing, find, bookmarks, history, keyboard shortcuts and accessible native
controls with correct safe areas. WALA's web APIs and OAuth are required compatibility cases.

Separate profile cookies, site storage, history, permissions, extension state and download
metadata. Private browsing uses temporary storage and does not restore private tabs.
Explicitly saved recordings remain user files. Browser-state cloud sync is not an initial
requirement and must not be silently introduced.

Use platform credentials/Keychain instead of custom password encryption. Require origin
matching, explicit save/autofill action and profile/private-mode policy. Never expose
managed credentials to Rustscript imports.

First extensions use a scoped Rustscript format with declared origins/capabilities,
explicit installation and revocation. Chrome/Firefox/Safari extension compatibility is
a separate milestone; do not claim it. Downloaded code and exposed native technologies
also need Apple policy assessment; a WASM sandbox does not establish eligibility.

Bound per-tab and total memory including engine resources, tracking and recording
buffers. Do not silently retain background camera access. Maintain pinned engine updates,
security patch intake, regression checks and clear release notes from the first public build.

## 7. Verification and acceptance

Gate B: instrument direct calls; test memory bounds, invalid/stale handles, page destruction,
redirects, origins, profiles, frames/workers, permission denial/revocation and nonblocking
delivery. A standalone native interpreter and JS mock are deliberately insufficient evidence.

Gate C: physical iPhone/iPad arbitrary starting orientation, translation/rotation/stop/return,
portrait/landscape, camera cover, low light, tracking loss, background/resume, all recording
combinations and interrupted recovery. Measure static drift, return error, orientation error,
latency, frame drops and timestamp alignment with device/light/route/quality conditions.
Targets: display latency p95 below50ms and video/pose alignment within one captured frame.
These targets do not promise survey-grade positional accuracy.

Gate D: WALA authentication/license, brush/pen/touch behavior and identity, two-client sync,
stale revisions and recovery under storage/network/quota/auth failures. Verify tracking makes
no authored changes and rejected imports retain originals. Run WALA's required subsystem
checks when changing it, including workspace tests/fmt/Clippy, web/edge builds and relevant
Node/browser/identity tests. Never deploy to production just to demonstrate the prototype.

Gate E: ordinary sites, profiles/private browsing, downloads, credential handling, tab restore,
accessibility, crashes and memory pressure. Ship macOS and iOS independently; macOS success
does not pass iOS execution, distribution or physical-device gates.

## 8. MacBook Codex kickoff

Read AGENTS.md, README.md, STATUS.md and FEASIBILITY.md. Run the portable checks and
pinned Servo build. Save real runtime evidence with commit, OS/device, feature flags,
page bundle identity, exact commands and measured results. Implement the small diagnostic
engine patch before adding native camera services. Keep original recordings, secrets,
artwork, native artifacts and vendor checkouts out of source control.

If the engine cannot run WALA, the direct page-native call cannot be installed safely,
or worldwide iOS distribution lacks a supported route, report that concrete blocker and
stop the affected gate. A revised engine, JS bridge or regional distribution requires a
new user decision. Do not conceal the failed gate behind a large browser shell.
