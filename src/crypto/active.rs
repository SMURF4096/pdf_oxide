//! Active provider registry — the single global [`CryptoProvider`]
//! every PDF operation routes through.
//!
//! There's exactly one active provider at a time. It's set at most
//! once (via [`set_provider`]); subsequent calls return
//! [`SetProviderError::AlreadySet`]. After a provider is in use, swapping
//! mid-process would be a soundness hazard for in-flight crypto state
//! (e.g., the FIPS module's per-process self-test).
//!
//! Default behaviour: if no provider has been registered when
//! [`active`] is first called, [`RustCryptoProvider`] (the
//! permissive Rust-only default) is installed lazily. FIPS
//! deployments call [`set_provider`] at process startup before any
//! PDF operation.
//!
//! [`set_provider`]: self::set_provider
//! [`active`]: self::active
//! [`RustCryptoProvider`]: super::RustCryptoProvider

use std::sync::{Arc, OnceLock};

use super::provider::CryptoProvider;
use super::rust_provider::RustCryptoProvider;

/// Errors from [`set_provider`].
#[derive(Debug)]
pub enum SetProviderError {
    /// A provider has already been installed (either by a prior call
    /// or by lazy default initialization on first [`active`] call).
    AlreadySet,
}

impl std::fmt::Display for SetProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetProviderError::AlreadySet => f.write_str(
                "crypto provider already set — set_provider() must be called \
                 once at process startup, before any PDF operation",
            ),
        }
    }
}

impl std::error::Error for SetProviderError {}

static ACTIVE: OnceLock<Arc<dyn CryptoProvider>> = OnceLock::new();

/// Install `provider` as the process-wide active [`CryptoProvider`].
///
/// Must be called before any PDF operation that uses crypto (open
/// encrypted document, verify signature, etc.). Returns
/// [`SetProviderError::AlreadySet`] if a provider is already
/// installed.
///
/// FIPS deployments call this once with `AwsLcProvider` (behind the
/// `fips` feature) at process startup.
///
/// Tests that need a fresh provider registry must run in their own
/// process — `cargo test` reuses one binary per crate target, so
/// `#[test]` functions in the same lib/integration target share this
/// `OnceLock`. Use a separate integration-test target (`tests/<name>.rs`
/// invoked via `cargo test --test <name>`) or a custom test harness so
/// each test gets its own process and a fresh `ACTIVE` cell.
pub fn set_provider(provider: Arc<dyn CryptoProvider>) -> Result<(), SetProviderError> {
    ACTIVE
        .set(provider)
        .map_err(|_| SetProviderError::AlreadySet)
}

/// Returns the active provider, lazily initializing
/// [`RustCryptoProvider`] on first call if none was registered.
pub fn active() -> &'static Arc<dyn CryptoProvider> {
    ACTIVE.get_or_init(|| Arc::new(RustCryptoProvider::new()))
}

/// Reports whether a provider has been installed (either explicitly
/// via [`set_provider`] or lazily by a previous [`active`] call).
pub fn is_set() -> bool {
    ACTIVE.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::super::HashAlgorithm;
    use super::*;

    /// `active()` must succeed for a fresh process and return the
    /// permissive default (which permits MD5).
    #[test]
    fn lazy_default_is_rust_crypto() {
        let p = active();
        assert!(p.is_legacy_allowed());
        assert_eq!(p.name(), "rust-crypto");
        // Sanity: MD5 hasher works under default provider.
        let mut h = p.hasher(HashAlgorithm::Md5).unwrap();
        h.update(b"abc");
        let out = h.finalize();
        assert_eq!(out.len(), 16);
    }

    /// Once active() has lazily installed the default,
    /// `set_provider` rejects any further attempt — proves the
    /// "set-at-most-once" invariant for downstream callers that rely
    /// on it.
    #[test]
    fn set_provider_after_lazy_init_fails() {
        let _ = active();
        let attempt = set_provider(Arc::new(RustCryptoProvider::new()));
        assert!(matches!(attempt, Err(SetProviderError::AlreadySet)));
    }
}
