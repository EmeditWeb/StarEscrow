/// Secure secret key storage via the OS keychain (issue #142).
///
/// Uses the `keyring` crate to store and retrieve Stellar secret keys from
/// the platform keychain (macOS Keychain, Linux Secret Service, Windows
/// Credential Manager).
///
/// # Security implications
///
/// - Keys stored here are protected by the OS keychain, which is significantly
///   more secure than plain `.env` files on disk.
/// - The keychain entry is scoped to the current user account.
/// - On Linux, the Secret Service daemon (e.g. GNOME Keyring or KWallet) must
///   be running; if unavailable the functions return `None` and the caller
///   falls back to environment variables.
/// - Secret keys are never written to disk by this module; they live only in
///   the OS keychain and in process memory while in use.
/// - Callers should zeroize secret key strings after use where possible.
use anyhow::Result;

const SERVICE: &str = "star-escrow";

/// Store a secret key in the OS keychain under the given name.
///
/// Returns `Ok(())` on success, or an error if the keychain is unavailable.
pub fn store(name: &str, secret: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, name)?;
    entry.set_password(secret)?;
    Ok(())
}

/// Retrieve a secret key from the OS keychain by name.
///
/// Returns `Some(secret)` if found, `None` if the entry does not exist or
/// the keychain is unavailable (allowing callers to fall back to env vars).
pub fn load(name: &str) -> Option<String> {
    let entry = keyring::Entry::new(SERVICE, name).ok()?;
    entry.get_password().ok()
}

/// Delete a secret key from the OS keychain by name.
pub fn delete(name: &str) -> Result<()> {
    let entry = keyring::Entry::new(SERVICE, name)?;
    entry.delete_password()?;
    Ok(())
}

/// Resolve a secret key: try the keychain first, then fall back to the
/// provided environment variable value.
///
/// This is the primary entry point used by CLI commands.
pub fn resolve(keychain_name: &str, env_value: Option<&str>) -> Option<String> {
    load(keychain_name).or_else(|| env_value.map(str::to_owned))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `resolve` returns the env value when keychain has no entry.
    #[test]
    fn test_resolve_falls_back_to_env() {
        // Use a name that is very unlikely to exist in any keychain.
        let name = "star-escrow-test-nonexistent-key-xyz";
        let env_val = "STEST_SECRET_VALUE";
        let result = resolve(name, Some(env_val));
        // Either the keychain returned something (unlikely in CI) or we got the env fallback.
        assert!(result.is_some());
        // If keychain had nothing, we must get the env value.
        if load(name).is_none() {
            assert_eq!(result.unwrap(), env_val);
        }
    }

    /// Verify that `resolve` returns `None` when both keychain and env are absent.
    #[test]
    fn test_resolve_none_when_both_absent() {
        let name = "star-escrow-test-nonexistent-key-xyz";
        let result = resolve(name, None);
        if load(name).is_none() {
            assert!(result.is_none());
        }
    }

    /// Verify that `load` returns `None` (not a panic) when keychain is unavailable.
    #[test]
    fn test_load_missing_key_returns_none() {
        // This should never panic regardless of keychain availability.
        let _ = load("star-escrow-test-definitely-missing-key");
    }
}
