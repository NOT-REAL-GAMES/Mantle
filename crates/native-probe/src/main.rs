//! A native WASM test bench, not the browser's page runtime or Gate A proof.

use rustscript_core::{Broker, Capability, Handle, PageToken};
use std::{env, fs::File, io::Read, path::Path, process::ExitCode};
use wasmi::{
    Caller, Config, EnforcedLimits, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder,
};

const MAX_MODULE_BYTES: usize = 1024 * 1024;
const MAX_MEMORY_BYTES: usize = 32 * 1024 * 1024;
const FUEL: u64 = 200_000;
const IMPORTS: [&str; 3] = ["abi_version", "capabilities", "probe_add"];

struct Host {
    limits: StoreLimits,
    broker: Broker,
    page: PageToken,
    handle: Handle,
    calls: [u32; 3],
    calls_in_order: bool,
    total_calls: u32,
}

impl Host {
    fn called(&mut self, index: usize) -> Result<(), wasmi::Error> {
        self.broker
            .validate(self.page, self.handle, Capability::Diagnostic)
            .map_err(|error| {
                wasmi::Error::new(format!("Native diagnostic authority denied: {error:?}"))
            })?;
        self.calls_in_order &= self.total_calls as usize == index;
        self.total_calls += 1;
        self.calls[index] += 1;
        Ok(())
    }
}

fn test_bench_host() -> Result<Host, String> {
    // Artificial test-bench identity and approval; no browser realm or user permission is proven.
    let mut broker = Broker::new(Capability::Diagnostic as u32);
    let page = broker
        .navigate(1, 1, "https://mantle.example/", true, false)
        .map_err(|error| format!("Cannot create test-bench context: {error:?}"))?;
    let activation = broker
        .activate(page)
        .map_err(|error| format!("Cannot activate test-bench context: {error:?}"))?;
    let handle = broker
        .approve(page, Capability::Diagnostic, activation)
        .map_err(|error| format!("Cannot grant test-bench diagnostic: {error:?}"))?;
    Ok(Host {
        limits: StoreLimitsBuilder::new()
            .memory_size(MAX_MEMORY_BYTES)
            .memories(1)
            .instances(1)
            .tables(1)
            .table_elements(4096)
            .trap_on_grow_failure(true)
            .build(),
        broker,
        page,
        handle,
        calls: [0; 3],
        calls_in_order: true,
        total_calls: 0,
    })
}

#[derive(Debug)]
struct Evidence {
    result: i32,
    host_calls: [u32; 3],
    fuel_consumed: u64,
    module_bytes: usize,
}

impl Evidence {
    fn json(&self) -> String {
        format!(
            concat!(
                "{{\"evidenceScope\":\"standalone-native-runtime\",",
                "\"authorityScope\":\"artificial-test-bench-context\",",
                "\"browserPageRuntimeVerified\":false,\"noJavaScriptTransport\":true,",
                "\"runtime\":\"wasmi-2.0.0\",\"abiVersion\":1,\"capabilities\":1,",
                "\"result\":{},\"nativeHostCalls\":{},",
                "\"hostCalls\":{{\"abi_version\":{},\"capabilities\":{},\"probe_add\":{}}},",
                "\"fuelConsumed\":{},\"fuelLimit\":{},",
                "\"moduleBytes\":{},\"moduleByteLimit\":{},\"memoryByteLimit\":{}}}"
            ),
            self.result,
            self.host_calls.iter().sum::<u32>(),
            self.host_calls[0],
            self.host_calls[1],
            self.host_calls[2],
            self.fuel_consumed,
            FUEL,
            self.module_bytes,
            MAX_MODULE_BYTES,
            MAX_MEMORY_BYTES,
        )
    }
}

fn read_module(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("Cannot open WASM module: {error}"))?;
    // Read at most one excess byte, including when a file grows after it is opened.
    let mut bytes = Vec::new();
    file.take((MAX_MODULE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Cannot read WASM module: {error}"))?;
    Ok(bytes)
}

fn run_module(bytes: &[u8]) -> Result<Evidence, String> {
    run_module_with_host(bytes, test_bench_host()?)
}

fn run_module_with_host(bytes: &[u8], host: Host) -> Result<Evidence, String> {
    if bytes.len() > MAX_MODULE_BYTES {
        return Err("WASM module exceeds the 1 MiB limit".into());
    }
    if !bytes.starts_with(b"\0asm\x01\0\0\0") {
        return Err("Expected a binary WebAssembly version 1 module".into());
    }

    let mut config = Config::default();
    config
        .consume_fuel(true)
        .allow_start_fn(false)
        .enforced_limits(EnforcedLimits::strict())
        .set_max_recursion_depth(64)
        .set_max_stack_height(256 * 1024);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, bytes).map_err(|error| error.to_string())?;
    let mut seen = [false; 3];
    for import in module.imports() {
        let Some(index) = IMPORTS.iter().position(|name| *name == import.name()) else {
            return Err("Module requests an unexpected import".into());
        };
        if import.module() != "rustscript_v1" || seen[index] {
            return Err("Module requests an unexpected or duplicate import".into());
        }
        seen[index] = true;
    }
    if seen != [true; 3] {
        return Err("Module must import all three Rustscript diagnostic functions".into());
    }

    let mut store = Store::new(&engine, host);
    store.limiter(|host| &mut host.limits);
    store.set_fuel(FUEL).map_err(|error| error.to_string())?;
    let mut linker = Linker::new(&engine);
    linker
        .func_wrap(
            "rustscript_v1",
            "abi_version",
            |mut caller: Caller<'_, Host>| -> Result<u32, wasmi::Error> {
                caller.data_mut().called(0)?;
                Ok(1)
            },
        )
        .map_err(|error| error.to_string())?;
    linker
        .func_wrap(
            "rustscript_v1",
            "capabilities",
            |mut caller: Caller<'_, Host>| -> Result<u32, wasmi::Error> {
                caller.data_mut().called(1)?;
                Ok(1)
            },
        )
        .map_err(|error| error.to_string())?;
    linker
        .func_wrap(
            "rustscript_v1",
            "probe_add",
            |mut caller: Caller<'_, Host>, lhs: i32, rhs: i32| -> Result<i32, wasmi::Error> {
                caller.data_mut().called(2)?;
                Ok(lhs.wrapping_add(rhs))
            },
        )
        .map_err(|error| error.to_string())?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|error| error.to_string())?;
    let result = instance
        .get_typed_func::<(), i32>(&store, "run_probe")
        .and_then(|function| function.call(&mut store, ()))
        .map_err(|error| error.to_string())?;
    if result != 42 {
        return Err(format!(
            "Guest diagnostic failed with result {result}; expected 42"
        ));
    }
    let host = store.data();
    if host.calls != [1; 3] || !host.calls_in_order || host.total_calls != 3 {
        return Err("Expected one native call each, in order: ABI, capabilities, addition".into());
    }
    Ok(Evidence {
        result,
        host_calls: host.calls,
        fuel_consumed: FUEL - store.get_fuel().map_err(|error| error.to_string())?,
        module_bytes: bytes.len(),
    })
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(path) = args.next() else {
        eprintln!("Usage: native-probe <bridge-probe.wasm>");
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!("Usage: native-probe <bridge-probe.wasm>");
        return ExitCode::FAILURE;
    }
    match read_module(Path::new(&path)).and_then(|bytes| run_module(&bytes)) {
        Ok(evidence) => {
            println!("{}", evidence.json());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Native probe failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guest(prefix: &str, body: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module
                (import "rustscript_v1" "abi_version" (func $abi (result i32)))
                (import "rustscript_v1" "capabilities" (func $caps (result i32)))
                (import "rustscript_v1" "probe_add" (func $add (param i32 i32) (result i32)))
                {prefix}
                (func (export "run_probe") (result i32) {body})
            )"#
        ))
        .unwrap()
    }

    const VALID_BODY: &str = r#"
        call $abi i32.const 1 i32.ne
        if i32.const -2 return end
        call $caps i32.const 1 i32.and i32.eqz
        if i32.const -3 return end
        i32.const 20 i32.const 22 call $add
    "#;

    #[test]
    fn native_calls_have_honest_evidence() {
        let report = run_module(&guest("", VALID_BODY)).unwrap();
        assert_eq!(report.host_calls, [1; 3]);
        assert!(report.fuel_consumed > 0);
        assert!(
            report
                .json()
                .contains("\"browserPageRuntimeVerified\":false")
        );
        assert!(report.json().contains("\"noJavaScriptTransport\":true"));
        assert!(report.json().contains("artificial-test-bench-context"));
    }

    #[test]
    fn native_calls_reject_revocation_close_and_navigation() {
        let actions: [fn(&mut Host); 3] = [
            |host| host.broker.revoke(host.handle),
            |host| host.broker.close(1, 1),
            |host| {
                host.broker
                    .navigate(1, 1, "https://other.example/", true, false)
                    .unwrap();
            },
        ];
        let bodies = [
            "call $abi drop i32.const 42",
            "call $caps drop i32.const 42",
            "i32.const 20 i32.const 22 call $add",
        ];
        for revoke in actions {
            for body in bodies {
                let mut host = test_bench_host().unwrap();
                revoke(&mut host);
                assert!(host.called(0).is_err());
                assert_eq!(host.calls, [0; 3]);
                assert!(
                    run_module_with_host(&guest("", body), host)
                        .unwrap_err()
                        .contains("authority denied")
                );
            }
        }
    }

    #[test]
    fn rejects_wrong_abi_missing_capability_and_forged_success() {
        let wrong_abi = VALID_BODY.replacen("i32.const 1 i32.ne", "i32.const 2 i32.ne", 1);
        assert!(
            run_module(&guest("", &wrong_abi))
                .unwrap_err()
                .contains("result -2")
        );
        let missing_capability =
            VALID_BODY.replacen("i32.const 1 i32.and", "i32.const 2 i32.and", 1);
        assert!(
            run_module(&guest("", &missing_capability))
                .unwrap_err()
                .contains("result -3")
        );
        assert!(
            run_module(&guest("", "i32.const 42"))
                .unwrap_err()
                .contains("native call")
        );
    }

    #[test]
    fn rejects_unexpected_imports_and_signatures() {
        let extra = r#"(import "wasi_snapshot_preview1" "fd_write" (func))"#;
        assert!(
            run_module(&guest(extra, VALID_BODY))
                .unwrap_err()
                .contains("unexpected import")
        );
        let wrong_signature = wat::parse_str(
            r#"(module
                (import "rustscript_v1" "abi_version" (func (result i64)))
                (import "rustscript_v1" "capabilities" (func (result i32)))
                (import "rustscript_v1" "probe_add" (func (param i32 i32) (result i32)))
                (func (export "run_probe") (result i32) i32.const 42))"#,
        )
        .unwrap();
        assert!(run_module(&wrong_signature).is_err());
    }

    #[test]
    fn memory_fuel_and_start_function_limits_are_enforced() {
        assert!(run_module(&guest("(memory 513)", VALID_BODY)).is_err());
        let growth = "i32.const 1024 memory.grow drop i32.const 42";
        assert!(run_module(&guest("(memory 1)", growth)).is_err());
        let loop_forever = "(loop $again br $again) i32.const 42";
        assert!(
            run_module(&guest("", loop_forever))
                .unwrap_err()
                .contains("fuel")
        );
        assert!(run_module(&guest("(func $start) (start $start)", VALID_BODY)).is_err());
    }

    #[test]
    fn rejects_nonbinary_and_oversized_modules() {
        assert!(run_module(b"(module)").unwrap_err().contains("binary"));
        assert!(
            run_module(&vec![0; MAX_MODULE_BYTES + 1])
                .unwrap_err()
                .contains("1 MiB")
        );
    }
}
