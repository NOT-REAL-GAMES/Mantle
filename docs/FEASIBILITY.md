# Mantle Gate A: engine, direct imports, and Apple distribution

Assessment date: 2026-10-06. This report records source inspection and the
remaining experiments. It is not evidence that an Apple browser binary or an
ARKit integration has been built or accepted.

## Decision and current status

Mantle keeps Servo as its first engine candidate and preserves the requirement that
native capability calls originate in the **page's WASM runtime**. Do not swap
that requirement for injected JavaScript, a WKWebView message handler, or a
separate native WASM interpreter.

| Gate A proof | Status | Evidence needed to pass |
| --- | --- | --- |
| macOS ordinary page, JavaScript, WASM, WebGPU | **Not run** | Build/run pinned Servo on the Mac; save runtime results and logs |
| Existing WALA in Servo | **Not run** | Actual editor, authentication, drawing, workers, storage, recovery |
| Page WASM directly calls native imports | **Blocked: engine integration absent** | Script/runtime patch and instrumented page execution |
| Origin/navigation isolation inside the engine | **Not run** | Real navigation, frame, worker, revocation, and lifetime tests |
| iOS engine binary and device execution | **Not run; port and entitlement work unresolved** | Signed physical-device build using an applicable supported route |
| Worldwide iOS App Store route | **Blocked by current published route** | Applicable Apple authorization or a new product decision |

A portable native VM exercise can test the ABI and host policy. It cannot pass
the browser engine or page-runtime rows. A JavaScript/WebGPU fixture that passes
in Chrome or Safari likewise cannot pass the Servo row.

## Exact engine inspected

Servo revision:
`964981f4c6eaa49a1f82700a4fb51afc76c43785`, committed
2026-10-06T08:28:40Z. The repository's engine pin is the build source of truth;
do not build against moving `main` and present the results as pinned results.

The revision's [README](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/README.md)
lists macOS, Linux, Windows, OpenHarmony, and Android development platforms. It
does not list iOS. This is evidence that an iOS port is unproven here, not a claim
that such a port is mathematically impossible. Its documented macOS route uses
Xcode, the upstream dependency bootstrap, and `mach build`.

The pinned [toolchain](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/rust-toolchain.toml)
specifies Rust 1.97.1. Use it for the Servo checkout. WALA's separate pinned
toolchain is not a reason to rewrite Servo's pin or combine the repositories.

The [Servo crate configuration](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/components/servo/Cargo.toml)
includes WebGPU among its default web features. Feature presence establishes
an implementation path to test; it does not establish that WALA's egui/wgpu
workload or all required browser APIs work.

## What the public embedder currently provides

The inspected public [WebView API](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/components/servo/webview.rs#L725-L741)
provides JavaScript evaluation and a user-content manager. The inspected
[UserContentManager](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/components/servo/user_content_manager.rs)
injects scripts and styles. Neither API supplies a native WASM import registry
in the inspected revision. Adding a script that calls a browser message bridge
would still be JavaScript transport.

Servo's private [script runtime](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/components/script/runtime/script_runtime.rs#L545-L598)
constructs the SpiderMonkey runtime through the `js` crate. The same file
[configures WASM execution](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/components/script/runtime/script_runtime.rs#L738-L747)
and contains CSP checks and streaming-compilation hooks. Runtime lifetime and
page execution are managed inside
[ScriptThread](https://github.com/servo/servo/blob/964981f4c6eaa49a1f82700a4fb51afc76c43785/components/script/event_loop/script_thread.rs).

**Engineering inference:** the direct import experiment requires work in
Servo's script/realm integration and possibly its SpiderMonkey bindings; the
existing public embedder script injection cannot demonstrate the requirement.
The exact native registration API must be established against the pinned
SpiderMonkey binding revision before writing the patch. No patch implementing
that registration is claimed by this assessment.

### Required direct-call experiment

1. Add an engine-owned, read-only import source for the `rustscript_v1`
   namespace, created in the relevant page realm. Its functions must be native
   host functions, not JavaScript forwarding closures.
2. Associate its host context with engine-derived origin, profile, browsing
   context, document identity, and runtime generation. Do not trust an origin
   string supplied by the page.
3. Instantiate the compiled test module in that **same page realm** with those
   imports. Ordinary JavaScript may load/instantiate a page's module; it must
   not handle any demonstrated native capability call.
4. Resolve WASM memory for each native call, validate pointer/length arithmetic,
   and copy only while its backing storage is safely accessible. Do not retain
   a raw pointer across growth, GC, reentry, or asynchronous completion.
5. Record a native entry count and browser stack/call-path evidence for the
   module's invocation. Prove that disabling/removing JavaScript message
   bridges does not disable that invocation.
6. Destroy/navigate the page and prove that pending completions and old handles
   fail. Repeat with two origins, a frame, private and regular profiles, and a
   revoked permission. First-version worker/frame requests must be denied if
   they are outside the supported top-level-page policy.

An import represented internally by a JavaScript engine function object is not
automatically a JavaScript adapter. The relevant distinction is whether it is
a native engine host function or executable JavaScript forwarding code. State
that distinction in the call-path report so the proof is reproducible.

Do not expose all native services at once. A side-effect-free native counter
and the already defined ABI are sufficient to prove the execution path before
connecting camera permissions or ARKit.

## iOS and iPadOS distribution assessment

Apple guideline 2.5.6 requires WebKit for web-browsing apps and identifies
alternative-engine entitlement routes for the EU and Japan. Guidelines 2.5.2
and 4.7.2 also require assessing downloaded code and native API exposure;
sandboxing WASM alone is not approval. Consent and recording indicators are
required. **Inference:** the proposed worldwide iOS custom-engine release has
no demonstrated published authorization route. Do not treat a developer build
or TestFlight submission as worldwide approval. [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/).

The EU route covers qualifying iOS/iPadOS apps for EU users, with minimum
versions iOS 17.4 and iPadOS 18. It requires an entitlement, at least 90% of the
specified WPT subtest baseline and 80% of Test262 on an eligible Apple device,
including the no-JIT case. It also imposes ongoing security, process
separation, privacy, and update obligations. None has been certified for this
project. [EU alternative engines](https://developer.apple.com/support/alternative-browser-engines/).

The Japan route describes iOS 26.2 and later and distribution in Japan under
its conditions, with corresponding functional and ongoing security
requirements. Its current platform wording is iOS; do not infer an iPadOS or
worldwide entitlement from it. [Japan alternative engines](https://developer.apple.com/support/alternative-browser-engines-jp/).

For both routes, permission to use JIT and multiple processes is a conditional
platform capability, not something a Rust target triple or browser shell
obtains automatically. The application's team, entitlements, signing,
distribution jurisdiction, process architecture, and engine qualification
need real validation on the Mac and physical devices.

### Routes that do not satisfy the selected requirement

| Alternative | Why it is not the accepted implementation |
| --- | --- |
| WKWebView plus script messages | Native calls travel through JavaScript transport |
| WKWebView plus an independent native WASM interpreter | The guest is outside the page's browser WASM runtime |
| Native WALA rendering instead of the web editor | Replaces the browser-only application boundary |
| A personal signed iPhone build | Does not establish App Store or worldwide distribution |
| A partial `navigator.xr` shim | Does not establish complete WebXR implementation or direct Rustscript imports |

Any change to those constraints needs a new product decision. Do not hide the
change behind the name Rustscript.

## Mac execution checklist

Use a separate checkout of the pinned Servo revision. Preserve this browser
repository, WALA, and the shared auth repository as separate projects.

1. Inspect Xcode selection, macOS version, architecture, Rustup, `uv`, and
   upstream build prerequisites. Bootstrap and build using the pinned
   checkout's documented commands. Record tool versions and exact commands.
2. Run the supplied browser fixture in **Servo**, saving its actual runtime
   report. Test a normal HTTPS page as well; a localhost fixture cannot prove
   production cookie, secure-context, certificate, or popup behavior.
3. Exercise the current WALA build: Gumroad sign-in, activated-license edit,
   paint/erase, pen/touch input, workers, IndexedDB save/reload, file export,
   two-client sync, and recoverable transport/storage errors. Keep artwork and
   authentication secrets out of source control and logs.
4. Implement and instrument the direct-call experiment above. Preserve a
   minimal engine patch, document the changed private interfaces, and test
   native callbacks on the correct script thread.
5. Investigate iOS cross-compilation without claiming upstream support. Check
   native dependencies, surface/input integration, WebGPU, SpiderMonkey/JIT
   execution, process separation, lifecycle, and applicable entitlements.
6. Record device execution and distribution as separate results. Seek Apple
   authorization for the selected route; never invent entitlement values or
   suggest bypassing platform restrictions.

For every proof record commit, machine/device, OS, engine feature flags, page
bundle identity, timestamp, commands, measured results, and logs. Mark a
blocked or failed result honestly. Unit tests, source inspection, and a
native-VM proof are useful evidence but do not replace the physical browser
checks.

## Stop conditions

Stop Gate A and report the concrete blocker if the pinned engine cannot run
WALA, if a page-runtime native import cannot be installed safely, or if the
required Apple execution/distribution route is unavailable. Do not launch an
engine rewrite, replace the engine, weaken origin isolation, or implement a
large browser shell to conceal a failed gate. Continue only with work that
tests the identified blocker or with an explicit revised product decision.
