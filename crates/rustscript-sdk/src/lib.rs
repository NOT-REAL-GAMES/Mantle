//! Minimal diagnostic ABI for a page's native Rustscript WASM imports.
//! No JavaScript adapter or standalone-runtime fallback is provided.

#![no_std]

use core::fmt;

pub const ABI_VERSION: u32 = 1;
pub const CAP_DIAGNOSTIC: u32 = 1;
pub const EXPECTED_RESULT: i32 = 42;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    UnsupportedPlatform,
    AbiMismatch { actual: u32 },
    MissingDiagnosticCapability,
    UnexpectedResult { actual: i32 },
}

impl Error {
    /// Stable diagnostic statuses exported by the guest probe.
    pub const fn status(self) -> i32 {
        match self {
            Self::UnsupportedPlatform => -1,
            Self::AbiMismatch { .. } => -2,
            Self::MissingDiagnosticCapability => -3,
            Self::UnexpectedResult { .. } => -4,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                write!(f, "Rustscript requires browser WASM native imports")
            }
            Self::AbiMismatch { actual } => {
                write!(
                    f,
                    "Rustscript ABI {actual} does not match required ABI {ABI_VERSION}"
                )
            }
            Self::MissingDiagnosticCapability => {
                write!(f, "Rustscript diagnostic capability absent")
            }
            Self::UnexpectedResult { actual } => {
                write!(
                    f,
                    "native diagnostic returned {actual}, expected {EXPECTED_RESULT}"
                )
            }
        }
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod host {
    #[link(wasm_import_module = "rustscript_v1")]
    unsafe extern "C" {
        pub fn abi_version() -> u32;
        pub fn capabilities() -> u32;
        pub fn probe_add(left: i32, right: i32) -> i32;
    }
}

#[cfg(any(all(target_arch = "wasm32", target_os = "unknown"), test))]
fn validate_abi(abi: u32) -> Result<(), Error> {
    if abi != ABI_VERSION {
        return Err(Error::AbiMismatch { actual: abi });
    }
    Ok(())
}

/// Inspect a native host without treating absent support as success.
pub fn capabilities() -> Result<u32, Error> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        // These imports take no pointers and return only ABI scalar values.
        unsafe {
            validate_abi(host::abi_version())?;
            Ok(host::capabilities())
        }
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        Err(Error::UnsupportedPlatform)
    }
}

fn validate_diagnostic_capability(capabilities: u32) -> Result<(), Error> {
    if capabilities & CAP_DIAGNOSTIC == 0 {
        return Err(Error::MissingDiagnosticCapability);
    }
    Ok(())
}

/// Call the diagnostic host function after checking the ABI and capability.
pub fn probe_add(left: i32, right: i32) -> Result<i32, Error> {
    validate_diagnostic_capability(capabilities()?)?;
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        // The diagnostic ABI passes scalars only; no native memory is exposed.
        Ok(unsafe { host::probe_add(left, right) })
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        let _ = (left, right);
        Err(Error::UnsupportedPlatform)
    }
}

fn validate_result(actual: i32) -> Result<i32, Error> {
    if actual != EXPECTED_RESULT {
        return Err(Error::UnexpectedResult { actual });
    }
    Ok(actual)
}

pub fn run_diagnostic() -> Result<i32, Error> {
    validate_result(probe_add(20, 22)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn native_target_reports_unsupported() {
        assert_eq!(capabilities(), Err(Error::UnsupportedPlatform));
        assert_eq!(probe_add(20, 22), Err(Error::UnsupportedPlatform));
        assert_eq!(run_diagnostic(), Err(Error::UnsupportedPlatform));
    }

    #[test]
    fn diagnostic_checks_classify_failures() {
        assert_eq!(validate_abi(ABI_VERSION), Ok(()));
        assert_eq!(validate_abi(2), Err(Error::AbiMismatch { actual: 2 }));
        assert_eq!(
            validate_diagnostic_capability(0),
            Err(Error::MissingDiagnosticCapability)
        );
        assert_eq!(validate_diagnostic_capability(CAP_DIAGNOSTIC | 128), Ok(()));
        assert_eq!(validate_result(42), Ok(42));
        assert_eq!(
            validate_result(41),
            Err(Error::UnexpectedResult { actual: 41 })
        );
        assert_eq!(Error::UnsupportedPlatform.status(), -1);
        assert_eq!(Error::AbiMismatch { actual: 2 }.status(), -2);
        assert_eq!(Error::MissingDiagnosticCapability.status(), -3);
        assert_eq!(Error::UnexpectedResult { actual: 41 }.status(), -4);
    }
}
