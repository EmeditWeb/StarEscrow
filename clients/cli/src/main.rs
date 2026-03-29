use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use serde_json::{json, Value};
use stellar_strkey::{ed25519, Strkey};
use ed25519_dalek::SigningKey;
use tabled::{Table, Tabled};

mod deadline;
mod keypair;
mod keystore;
mod native_xdr;
mod rpc;
mod wasm_hash;
mod xdr;

/// StarEscrow CLI — interact with the escrow contract on Stellar Testnet.
///
/// Prerequisites:
///   - Stellar CLI installed: https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli
///   - Contract deployed and ESCROW_CONTRACT_ID set in env
///   - PAYER_SECRET and FREELANCER_SECRET set in env
#[derive(Parser)]
#[command(name = "star-escrow", version, about)]
struct Cli {
    /// Path to a TOML config file. Defaults to ~/.star-escrow/config.toml.
    /// Config values are overridden by explicit CLI flags.
    #[arg(long, value_name = "FILE")]
    config: Option<std::path::PathBuf>,

    /// Network shorthand: testnet, mainnet, or futurenet.
    /// Sets --rpc-url and --network-passphrase automatically.
    /// Cannot be combined with --rpc-url or --network-passphrase.
    #[arg(long, value_enum, conflicts_with_all = ["rpc_url", "network_passphrase"])]
    network: Option<Network>,

    /// Soroban RPC endpoint. Defaults to testnet if neither --network nor --rpc-url is given.
    #[arg(long)]
    rpc_url: Option<String>,

    /// Network passphrase. Defaults to testnet if neither --network nor --network-passphrase is given.
    #[arg(long)]
    network_passphrase: Option<String>,

    /// Output results as JSON
    #[arg(long, global = true)]
    json: bool,

    /// Simulate the transaction without submitting it to the network.
    /// Uses Soroban's simulation endpoint to preview the transaction
    /// outcome.
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

/// TOML config file format:
///
/// ```toml
/// rpc_url = "https://soroban-testnet.stellar.org"
/// network_passphrase = "Test SDF Network ; September 2015"
/// contract_id = "C..."
/// ```
#[derive(Debug, Default, Deserialize, Serialize)]
struct ConfigFile {
    rpc_url: Option<String>,
    network_passphrase: Option<String>,
    contract_id: Option<String>,
}

impl ConfigFile {
    fn load(path: &std::path::Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading config file {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing config file {}", path.display()))
    }

    fn load_default_or_explicit(explicit: Option<&std::path::Path>) -> Result<Self> {
        let path = match explicit {
            Some(p) => p.to_path_buf(),
            None => {
                let home = std::env::var("HOME").unwrap_or_default();
                std::path::PathBuf::from(home)
                    .join(".star-escrow")
                    .join("config.toml")
            },
        };
        if path.exists() {
            Self::load(&path)
        } else if explicit.is_some() {
            anyhow::bail!("config file not found: {}", path.display());
        } else {
            Ok(Self::default())
        }
    }
}

#[derive(clap::ValueEnum, Clone)]
enum Network {
    Testnet,
    Mainnet,
    Futurenet,
}

impl Network {
    fn rpc_url(&self) -> &'static str {
        match self {
            Network::Testnet => "https://soroban-testnet.stellar.org",
            Network::Mainnet => "https://soroban-mainnet.stellar.org",
            Network::Futurenet => "https://rpc-futurenet.stellar.org",
        }
    }

    fn passphrase(&self) -> &'static str {
        match self {
            Network::Testnet => "Test SDF Network ; September 2015",
            Network::Mainnet => "Public Global Stellar Network ; September 2015",
            Network::Futurenet => "Test SDF Future Network ; October 2022",
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise protocol config (admin, fee)
    Init {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "ADMIN_SECRET")]
        admin_secret: String,
        /// Fee in basis points (e.g. 100 = 1%)
        #[arg(long, default_value = "0")]
        fee_bps: u32,
        /// Fee collector Stellar address
        #[arg(long)]
        fee_collector: String,
    },
    /// Pause all state-changing operations (admin only)
    Pause {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "ADMIN_SECRET")]
        admin_secret: String,
    },
    /// Unpause the contract (admin only)
    Unpause {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "ADMIN_SECRET")]
        admin_secret: String,
    },
    /// Create a new escrow and lock funds
    Create {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
        #[arg(long)]
        freelancer: String,
        #[arg(long)]
        token: String,
        #[arg(long)]
        amount: i128,
        #[arg(long)]
        milestone: String,
        /// Deadline as ISO 8601 (e.g. "2026-12-31T23:59:59Z") or Unix timestamp (seconds).
        #[arg(long)]
        deadline: Option<String>,
    },
    /// Freelancer submits work
    SubmitWork {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "FREELANCER_SECRET")]
        freelancer_secret: String,
    },
    /// Transfer freelancer role to a new address
    TransferFreelancer {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "FREELANCER_SECRET")]
        freelancer_secret: String,
        #[arg(long)]
        new_freelancer: String,
    },
    /// Payer approves milestone and releases payment
    Approve {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
    },
    /// Payer cancels escrow and gets refund (only before work submitted)
    Cancel {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
    },
    /// Payer reclaims funds after the deadline has passed
    Expire {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long, env = "PAYER_SECRET")]
        payer_secret: String,
    },
    /// Read current escrow status and full data
    Status {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        /// Token address to include balance in output
        #[arg(long)]
        token: Option<String>,
    },
    /// List all escrows created by a payer address
    List {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,
        #[arg(long)]
        payer: String,
    },

    /// Build (optional) and deploy the escrow contract WASM to the
    /// network
    Deploy {
        /// Path to pre-built WASM file. If omitted, runs `stellar
        /// contract build` first.
        #[arg(long)]
        wasm: Option<std::path::PathBuf>,

        /// Deployer secret key (pays the deployment fee)
        #[arg(long, env = "DEPLOYER_SECRET")]
        deployer_secret: String,

        /// Write the resulting contract ID to a local .env file
        #[arg(long, default_value = ".env")]
        env_file: std::path::PathBuf,
    },

    /// Verify a local WASM file's SHA-256 hash against the on-chain deployed hash
    Verify {
        #[arg(long, env = "ESCROW_CONTRACT_ID")]
        contract_id: String,

        /// Path to the local WASM file to verify
        #[arg(long)]
        wasm: std::path::PathBuf,

        /// Only print the local hash without fetching on-chain (offline mode)
        #[arg(long)]
        local_only: bool,
    },

    /// Manage keypairs stored in the OS keychain (issue #142)
    Keypair {
        #[command(subcommand)]
        action: KeypairAction,
    },
}

/// Subcommands for `keypair`.
#[derive(Subcommand)]
enum KeypairAction {
    /// Store a secret key in the OS keychain
    Store {
        /// Logical name for the key (e.g. "payer", "freelancer")
        #[arg(long)]
        name: String,
        /// The Stellar secret key (S...) to store
        #[arg(long)]
        secret: String,
    },
    /// Retrieve a secret key from the OS keychain
    Get {
        /// Logical name of the key to retrieve
        #[arg(long)]
        name: String,
    },
    /// Delete a secret key from the OS keychain
    Delete {
        /// Logical name of the key to delete
        #[arg(long)]
        name: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let as_json = cli.json;
    let dry_run = cli.dry_run;

    let cfg = ConfigFile::load_default_or_explicit(cli.config.as_deref())?;

    let (rpc_url, network_passphrase) = match &cli.network {
        Some(net) => {
            (net.rpc_url().to_string(), net.passphrase().to_string())
        },
        None => (
            cli.rpc_url
                .clone()
                .or(cfg.rpc_url.clone())
                .unwrap_or_else(|| "https://soroban-testnet.stellar.org".to_string()),
            cli.network_passphrase
                .clone()
                .or(cfg.network_passphrase.clone())
                .unwrap_or_else(|| "Test SDF Network ; September 2015".to_string()),
        ),
    };

    match cli.command {
        Commands::Init {
            contract_id,
            admin_secret,
            fee_bps,
            fee_collector,
        } => {
            let admin_addr = stellar_address_from_secret(&admin_secret)?;
            invoke_stellar_cli(
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &admin_secret,
                "init",
                &[
                    "--admin",
                    &admin_addr,
                    "--fee-bps",
                    &fee_bps.to_string(),
                    "--fee-collector",
                    &fee_collector,
                ],
                dry_run,
                as_json,
            )?;
            if !dry_run {
                output(
                    as_json,
                    json!({"status": "ok", "action": "init"}),
                    "Protocol initialised.",
                );
            }
        },
        Commands::Pause {
            contract_id,
            admin_secret,
        } => {
            invoke_stellar_cli(
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &admin_secret,
                "pause",
                &[],
                dry_run,
                as_json,
            )?;
            if !dry_run {
                output(
                    as_json,
                    json!({"status": "ok", "action": "pause"}),
                    "Contract paused.",
                );
            }
        },
        Commands::Unpause {
            contract_id,
            admin_secret,
        } => {
            invoke_stellar_cli(
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &admin_secret,
                "unpause",
                &[],
                dry_run,
                as_json,
            )?;
            if !dry_run {
                output(
                    as_json,
                    json!({"status": "ok", "action": "unpause"}),
                    "Contract unpaused.",
                );
            }
        },
        Commands::Create {
            contract_id,
            payer_secret,
            freelancer,
            token,
            amount,
            milestone,
            deadline,
        } => {
            let payer_addr = stellar_address_from_secret(&payer_secret)?;
            // Accept ISO 8601 string or raw Unix timestamp integer
            let deadline_ts: Option<u64> = match &deadline {
                None => None,
                Some(s) => {
                    let ts = s.parse::<u64>()
                        .ok()
                        .map(Ok)
                        .unwrap_or_else(|| deadline::parse_iso8601_to_timestamp(s))?;
                    Some(ts)
                }
            };
            let deadline_str = deadline_ts.map(|d| d.to_string()).unwrap_or_else(|| "null".into());
            let deadline_human = deadline_ts.map(|d| deadline::format_timestamp(d));
            invoke_stellar_cli(
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &payer_secret,
                "create",
                &[
                    "--payer",
                    &payer_addr,
                    "--freelancer",
                    &freelancer,
                    "--token",
                    &token,
                    "--amount",
                    &amount.to_string(),
                    "--milestone",
                    &milestone,
                    "--deadline",
                    &deadline_str,
                ],
                dry_run,
                as_json,
            )?;
            if !dry_run {
                output(
                    as_json,
                    json!({"status":"ok","action":"create","contract_id":contract_id,"payer":payer_addr,
                           "freelancer":freelancer,"amount":amount,"milestone":milestone,
                           "deadline_ts":deadline_ts,"deadline":deadline_human}),
                    &format!("Escrow created. Funds locked.{}",
                        deadline_human.as_deref().map(|d| format!(" Deadline: {d}")).unwrap_or_default()));
            }
        }
        Commands::SubmitWork { contract_id, freelancer_secret } => {
            invoke_stellar_cli(&rpc_url, &network_passphrase, &contract_id, &freelancer_secret, "submit_work", &[])?;
            output(as_json, json!({"status":"ok","action":"submit_work"}), "Work submitted. Waiting for payer approval.");
        },
        Commands::TransferFreelancer { contract_id, freelancer_secret, new_freelancer } => {
            invoke_stellar_cli(
                &rpc_url,
                &network_passphrase,
                &contract_id,
                &freelancer_secret,
                "transfer_freelancer",
                &["--new-freelancer", &new_freelancer],
                dry_run,
                as_json,
            )?;
            if !dry_run {
                output(
                    as_json,
                    json!({"status":"ok","action":"transfer_freelancer","new_freelancer":new_freelancer}),
                    &format!("Freelancer role transferred to {new_freelancer}."),
                );
            }
        },
        Commands::Approve { contract_id, payer_secret } => {
            invoke_stellar_cli(&rpc_url, &network_passphrase, &contract_id, &payer_secret, "approve", &[])?;
            output(as_json, json!({"status":"ok","action":"approve"}), "Payment released to freelancer.");
        },
        Commands::Cancel { contract_id, payer_secret } => {
            invoke_stellar_cli(&rpc_url, &network_passphrase, &contract_id, &payer_secret, "cancel", &[])?;
            output(as_json, json!({"status":"ok","action":"cancel"}), "Escrow cancelled. Funds refunded to payer.");
        },
        Commands::Expire { contract_id, payer_secret } => {
            invoke_stellar_cli(&rpc_url, &network_passphrase, &contract_id, &payer_secret, "expire", &[])?;
            output(as_json, json!({"status":"ok","action":"expire"}), "Escrow expired. Funds returned to payer.");
        },
        // Issue #149: tabular output for status
        Commands::Status { contract_id, token } => {
            let raw = query_contract(&rpc_url, &network_passphrase, &contract_id, "get_escrow")?;
            let balance: Option<String> = if let Some(ref tok) = token {
                let bal_raw = query_contract(&rpc_url, &network_passphrase, tok, "balance")?;
                Some(bal_raw.trim().to_string())
            } else {
                None
            };
            if as_json {
                let parsed: Value = serde_json::from_str(raw.trim())
                    .unwrap_or(Value::String(raw.trim().to_string()));
                println!("{}", serde_json::to_string_pretty(&json!({"status":"ok","escrow":parsed}))?);
            } else {
                print_status_table(raw.trim(), balance.as_deref());
            }
        },
        // Issue #149: tabular output for list
        Commands::List { contract_id, payer } => {
            list_escrows(&rpc_url, &network_passphrase, &contract_id, &payer, as_json)?;
        },
        Commands::Deploy { wasm, deployer_secret, env_file } => {
            deploy_contract(&rpc_url, &network_passphrase, wasm.as_deref(), &deployer_secret, &env_file, as_json)?;
        },
        Commands::Verify { contract_id, wasm, local_only } => {
            let local_hash = wasm_hash::hash_wasm_file(&wasm)?;
            if local_only {
                output(as_json, json!({"local_hash": local_hash}), &format!("Local hash: {local_hash}"));
            } else {
                let remote_hash = fetch_remote_wasm_hash(&rpc_url, &network_passphrase, &contract_id)?;
                let matches = local_hash == remote_hash;
                output(
                    as_json,
                    json!({"local_hash": local_hash, "remote_hash": remote_hash, "match": matches}),
                    &format!("Local:  {local_hash}\nRemote: {remote_hash}\nMatch:  {matches}"),
                );
            }
        },
        // Issue #142: keypair keychain management
        Commands::Keypair { action } => match action {
            KeypairAction::Store { name, secret } => {
                keystore::store(&name, &secret)?;
                output(as_json, json!({"status":"ok","action":"keypair_store","name":name}),
                    &format!("Secret key '{name}' stored in OS keychain."));
            },
            KeypairAction::Get { name } => {
                match keystore::load(&name) {
                    Some(secret) => output(as_json, json!({"status":"ok","name":name,"secret":secret}),
                        &format!("Secret key '{name}': {secret}")),
                    None => output(as_json, json!({"status":"not_found","name":name}),
                        &format!("No key found for '{name}' in OS keychain.")),
                }
            },
            KeypairAction::Delete { name } => {
                keystore::delete(&name)?;
                output(as_json, json!({"status":"ok","action":"keypair_delete","name":name}),
                    &format!("Secret key '{name}' deleted from OS keychain."));
            },
        },

        Commands::Verify { contract_id, wasm, local_only } => {
            run_verify(&rpc_url, &network_passphrase, &contract_id, &wasm, local_only)?;
        }
    }

    Ok(())
}

fn run_verify(
    rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
    wasm_path: &std::path::Path,
    local_only: bool,
) -> Result<()> {
    let mut hasher = Sha256::new();
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("Failed to read WASM at {}", wasm_path.display()))?;
    hasher.update(&wasm_bytes);
    let local_hash = hex::encode(hasher.finalize());

    println!("Local WASM hash: {local_hash}");

    if !local_only {
        print!("Fetching on-chain hash for {contract_id}... ");
        let remote_hash = fetch_remote_wasm_hash(rpc_url, network_passphrase, contract_id)?;
        println!("DONE");
        println!("On-chain hash:   {remote_hash}");

        if local_hash == remote_hash {
            println!("✅ VERIFICATION SUCCESS: Local WASM matches on-chain code.");
        } else {
            println!("❌ VERIFICATION FAILED: Hashes do not match!");
            anyhow::bail!("Hash mismatch");
        }
    }
    Ok(())
}

fn fetch_remote_wasm_hash(
    rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
) -> Result<String> {
    let out = std::process::Command::new("stellar")
        .args([
            "contract",
            "fetch",
            "--id",
            contract_id,
            "--rpc-url",
            rpc_url,
            "--network-passphrase",
            network_passphrase,
            "--output",
            "wasm",
        ])
        .output()
        .context("Failed to fetch contract WASM from network")?;

    if !out.status.success() {
        anyhow::bail!(
            "stellar contract fetch failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let mut hasher = Sha256::new();
    hasher.update(&out.stdout);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

fn deploy_contract(
    rpc_url: &str,
    network_passphrase: &str,
    wasm: Option<&std::path::Path>,
    deployer_secret: &str,
    env_file: &std::path::Path,
    as_json: bool,
    dry_run: bool,
) -> Result<()> {
    let wasm_path = match wasm {
        Some(p) => p.to_path_buf(),
        None => {
            eprintln!("No --wasm provided; running `stellar contract build`…");
            let status = std::process::Command::new("stellar")
                .args(["contract", "build"])
                .status()
                .context("stellar CLI not found")?;
            if !status.success() {
                anyhow::bail!("`stellar contract build` failed");
            }
            std::path::PathBuf::from("target/wasm32-unknown-unknown/release/escrow.wasm")
        },
    };

    if !wasm_path.exists() {
        anyhow::bail!("WASM file not found: {}", wasm_path.display());
    }

    let mut args = vec![
        "contract",
        "deploy",
        "--wasm",
        wasm_path.to_str().context("invalid wasm path")?,
        "--source",
        deployer_secret,
        "--rpc-url",
        rpc_url,
        "--network-passphrase",
        network_passphrase,
    ];

    if dry_run {
        args.push("--sim-only");
    }

    let out = std::process::Command::new("stellar")
        .args(&args)
        .output()
        .context("stellar CLI not found")?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("Deployment failed: {stderr}");
    }

    let contract_id = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if contract_id.is_empty() {
        anyhow::bail!("Deployment succeeded but no contract ID was returned");
    }

    if dry_run {
        output_dry_run(as_json, "deploy", &contract_id, &String::from_utf8_lossy(&out.stderr));
        return Ok(());
    }

    upsert_env_var(env_file, "ESCROW_CONTRACT_ID", &contract_id)?;

    output(
        as_json,
        serde_json::json!({"status": "ok", "contract_id": contract_id, "env_file": env_file.display().to_string()}),
        &format!("Deployed! Contract ID: {contract_id}\\nWritten to {}", env_file.display()),
    );
    Ok(())
}

fn upsert_env_var(path: &std::path::Path, key: &str, value: &str) -> Result<()> {
    use std::io::Write as _;
    let existing = if path.exists() {
        std::fs::read_to_string(path).context("reading .env file")?
    } else {
        String::new()
    };
    let prefix = format!("{key}=");
    let new_line = format!("{key}={value}");
    let mut found = false;
    let updated: String = existing.lines().map(|line| {
        if line.starts_with(&prefix) { found = true; new_line.clone() } else { line.to_string() }
    }).collect::<Vec<_>>().join("\n");
    let mut content = if found { updated } else { format!("{existing}\n{new_line}") };
    if !content.ends_with('\n') { content.push('\n'); }
    let mut file = std::fs::File::create(path).context("writing .env file")?;
    file.write_all(content.as_bytes()).context("writing .env file")?;
    Ok(())
}

fn stellar_address_from_secret(secret: &str) -> Result<String> {
    let out = std::process::Command::new("stellar")
        .args(["keys", "address", secret])
        .output()
        .context("stellar CLI not found")?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn invoke_stellar_cli(
    rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
    secret: &str,
    function: &str,
    extra_args: &[&str],
    dry_run: bool,
    as_json: bool,
) -> Result<()> {
    let mut args = vec![
        "contract",
        "invoke",
        "--id",
        contract_id,
        "--rpc-url",
        rpc_url,
        "--network-passphrase",
        network_passphrase,
        "--source",
        secret,
    ];
    if dry_run { args.push("--sim-only"); }
    args.push("--");
    args.push(function);
    args.extend_from_slice(extra_args);

    let out = if dry_run {
        std::process::Command::new("stellar").args(&args).output()?
    } else {
        let status = std::process::Command::new("stellar").args(&args).status()?;
        if !status.success() { anyhow::bail!("stellar CLI failed"); }
        return Ok(());
    };

    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    output_dry_run(as_json, function, &stdout, &stderr);
    if !out.status.success() { anyhow::bail!("Simulation failed"); }
    Ok(())
}

fn query_contract(rpc_url: &str, network_passphrase: &str, contract_id: &str, function: &str) -> Result<String> {
    let out = std::process::Command::new("stellar").args(["contract", "invoke", "--id", contract_id, "--rpc-url", rpc_url, "--network-passphrase", network_passphrase, "--", function]).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn query_contract_with_args(rpc_url: &str, network_passphrase: &str, contract_id: &str, function: &str, args: &[&str]) -> Result<String> {
    let mut cmd_args = vec!["contract", "invoke", "--id", contract_id, "--rpc-url", rpc_url, "--network-passphrase", network_passphrase, "--", function];
    cmd_args.extend_from_slice(args);
    let out = std::process::Command::new("stellar").args(&cmd_args).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn list_escrows(rpc_url: &str, network_passphrase: &str, contract_id: &str, payer: &str, as_json: bool) -> Result<()> {
    let events = fetch_events(rpc_url, network_passphrase, contract_id)?;
    let escrows: Vec<Value> = events.into_iter()
        .filter(|e| e["topic"][0].as_str().unwrap_or("") == "escrow_created" && e["value"][0].as_str().unwrap_or("") == payer)
        .map(|e| json!({"contract_id": contract_id,"payer": e["value"][0],"freelancer": e["value"][1],"amount": e["value"][2],"milestone": e["value"][3]}))
        .collect();
    if as_json {
        println!("{}", serde_json::to_string_pretty(&json!({"escrows": escrows}))?);
    } else if escrows.is_empty() {
        println!("No escrows found for payer {payer}");
    } else {
        // Issue #149: tabular output
        println!("Escrows for payer {payer}:");
        print_escrow_table(&escrows);
    }
    Ok(())
}

fn fetch_events(rpc_url: &str, network_passphrase: &str, contract_id: &str) -> Result<Vec<Value>> {
    let out = std::process::Command::new("stellar").args(["contract", "events", "--id", contract_id, "--rpc-url", rpc_url, "--network-passphrase", network_passphrase, "--output", "json"]).output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap_or_default())
}

#[allow(dead_code)]
fn fetch_remote_wasm_hash(
    rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
) -> Result<String> {
    wasm_hash::fetch_onchain_hash(rpc_url, network_passphrase, contract_id)
}

fn query_contract(
    rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
    function: &str,
) -> Result<String> {
    let args = [
        "contract", "invoke",
        "--id", contract_id,
        "--rpc-url", rpc_url,
        "--network-passphrase", network_passphrase,
        "--", function,
    ];
    let out = std::process::Command::new("stellar")
        .args(args)
        .output()
        .context("stellar CLI not found — install from https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli")?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

// ---------------------------------------------------------------------------
// Issue #149: tabular output helpers
// ---------------------------------------------------------------------------

/// A single key-value row for the status table.
#[derive(Tabled)]
struct StatusRow {
    #[tabled(rename = "Field")]
    field: String,
    #[tabled(rename = "Value")]
    value: String,
}

/// Print escrow status as a table (or fall back to raw text if unparseable).
fn print_status_table(raw: &str, balance: Option<&str>) {
    let parsed: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => {
            println!("{raw}");
            if let Some(bal) = balance {
                println!("balance: {bal}");
            }
            return;
        }
    };

    let obj = match parsed.as_object() {
        Some(o) => o,
        None => {
            println!("{raw}");
            return;
        }
    };

    let mut rows: Vec<StatusRow> = obj
        .iter()
        .map(|(k, v)| StatusRow {
            field: k.clone(),
            value: match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            },
        })
        .collect();

    if let Some(bal) = balance {
        rows.push(StatusRow { field: "balance".to_owned(), value: bal.to_owned() });
    }

    println!("{}", Table::new(rows));
}

/// A single row for the escrow list table.
#[derive(Tabled)]
struct EscrowRow {
    #[tabled(rename = "#")]
    index: usize,
    #[tabled(rename = "Contract ID")]
    contract_id: String,
    #[tabled(rename = "Milestone")]
    milestone: String,
    #[tabled(rename = "Amount")]
    amount: String,
    #[tabled(rename = "Freelancer")]
    freelancer: String,
}

/// Print a list of escrows as a table.
fn print_escrow_table(escrows: &[Value]) {
    let rows: Vec<EscrowRow> = escrows
        .iter()
        .enumerate()
        .map(|(i, e)| EscrowRow {
            index: i + 1,
            contract_id: e["contract_id"].as_str().unwrap_or("-").to_owned(),
            milestone: e["milestone"].as_str().unwrap_or("-").to_owned(),
            amount: e["amount"].to_string(),
            freelancer: e["freelancer"].as_str().unwrap_or("-").to_owned(),
        })
        .collect();
    println!("{}", Table::new(rows));
}

fn invoke_stellar_cli(
    rpc_url: &str,
    network_passphrase: &str,
    contract_id: &str,
    secret: &str,
    function: &str,
    extra_args: &[&str],
) -> Result<()> {
    let mut args = vec![
        "contract",
        "invoke",
        "--id",
        contract_id,
        "--rpc-url",
        rpc_url,
        "--network-passphrase",
        network_passphrase,
        "--source",
        secret,
        "--",
        function,
    ];
    args.extend_from_slice(extra_args);
    let status = std::process::Command::new("stellar")
        .args(&args)
        .status()
        .context("stellar CLI not found — install from https://developers.stellar.org/docs/tools/developer-tools/cli/install-cli")?;
    if !status.success() {
        anyhow::bail!("stellar CLI exited with status {status}");
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[test] fn test_dry_run_flag_parsing() {
        let cli = Cli::try_parse_from(["star-escrow", "--dry-run", "status", "--contract-id", "C1"]).unwrap();
        assert!(cli.dry_run);
    }
}
