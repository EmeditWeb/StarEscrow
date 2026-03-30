//! Issue #141 — Hierarchical configuration using the `config` crate.
//!
//! Priority (lowest → highest):
//!   1. Config file  (~/.star-escrow/config.toml or --config <FILE>)
//!   2. Environment variables  (STAR_ESCROW_RPC_URL, STAR_ESCROW_CONTRACT_ID, …)
//!   3. CLI flags  (resolved by clap, merged in main)
//!
//! Config file format (TOML):
//! ```toml
//! rpc_url            = "https://soroban-testnet.stellar.org"
//! network_passphrase = "Test SDF Network ; September 2015"
//! contract_id        = "C..."
//! ```
//!
//! Environment variables mirror the TOML keys with a `STAR_ESCROW_` prefix
//! and upper-case, e.g. `STAR_ESCROW_RPC_URL`.  The legacy bare names
//! (`ESCROW_CONTRACT_ID`, `PAYER_SECRET`, etc.) are still honoured by clap's
//! `env = "…"` attributes on individual subcommand arguments.

use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

/// Resolved configuration values loaded from file + env vars.
/// CLI flags are merged on top of this in `main`.
#[derive(Debug, Default, Deserialize)]
pub struct AppConfig {
    pub rpc_url: Option<String>,
    pub network_passphrase: Option<String>,
    pub contract_id: Option<String>,
}

impl AppConfig {
    /// Load config from file (if present) then overlay env vars.
    ///
    /// `explicit_path` is the value of `--config <FILE>` if provided.
    pub fn load(explicit_path: Option<&std::path::Path>) -> Result<Self> {
        let default_path = {
            let home = std::env::var("HOME").unwrap_or_default();
            std::path::PathBuf::from(home)
                .join(".star-escrow")
                .join("config.toml")
        };

        let file_path = explicit_path.unwrap_or(&default_path);

        let mut builder = Config::builder();

        // Layer 1: config file (optional unless explicitly requested)
        if file_path.exists() {
            builder = builder.add_source(
                File::from(file_path)
                    .format(FileFormat::Toml)
                    .required(explicit_path.is_some()),
            );
        } else if explicit_path.is_some() {
            anyhow::bail!("config file not found: {}", file_path.display());
        }

        // Layer 2: environment variables with STAR_ESCROW_ prefix
        builder = builder.add_source(
            Environment::with_prefix("STAR_ESCROW")
                .separator("_")
                .ignore_empty(true),
        );

        builder
            .build()
            .context("building config")?
            .try_deserialize()
            .context("deserialising config")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_defaults_when_no_file() {
        // No file, no env vars set — should return all-None defaults.
        let cfg = AppConfig::load(None).unwrap();
        // We can't assert None because CI may have STAR_ESCROW_* vars set,
        // but we can assert the call succeeds.
        let _ = cfg;
    }

    #[test]
    fn test_env_var_overrides() {
        std::env::set_var("STAR_ESCROW_RPC_URL", "https://example.com");
        let cfg = AppConfig::load(None).unwrap();
        assert_eq!(cfg.rpc_url.as_deref(), Some("https://example.com"));
        std::env::remove_var("STAR_ESCROW_RPC_URL");
    }

    #[test]
    fn test_explicit_missing_file_errors() {
        let result = AppConfig::load(Some(std::path::Path::new("/nonexistent/config.toml")));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_toml_file() {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        writeln!(f, r#"rpc_url = "https://test.example.com""#).unwrap();
        let cfg = AppConfig::load(Some(f.path())).unwrap();
        assert_eq!(cfg.rpc_url.as_deref(), Some("https://test.example.com"));
    }
}
