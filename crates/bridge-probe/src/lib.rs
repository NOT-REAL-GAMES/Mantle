//! A WASM guest with three native host imports and no JavaScript glue.

/// Returns 42 on success, or the SDK's stable negative diagnostic status.
#[unsafe(no_mangle)]
pub extern "C" fn run_probe() -> i32 {
    rustscript_sdk::run_diagnostic().unwrap_or_else(rustscript_sdk::Error::status)
}

#[unsafe(no_mangle)]
pub extern "C" fn expected_abi_version() -> u32 {
    rustscript_sdk::ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn diagnostic_capability() -> u32 {
    rustscript_sdk::CAP_DIAGNOSTIC
}

#[unsafe(no_mangle)]
pub extern "C" fn expected_result() -> i32 {
    rustscript_sdk::EXPECTED_RESULT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn native_guest_is_explicitly_unsupported() {
        assert_eq!(run_probe(), -1);
        assert_eq!(expected_abi_version(), 1);
        assert_eq!(diagnostic_capability(), 1);
        assert_eq!(expected_result(), 42);
    }
}
