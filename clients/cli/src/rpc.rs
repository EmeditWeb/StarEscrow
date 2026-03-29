/// Direct Soroban JSON-RPC client using `reqwest` (issue #143).
///
/// Implements the three RPC methods needed to submit and track transactions:
///   - `simulateTransaction`
///   - `sendTransaction`
///   - `getTransaction` (with polling)
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A minimal blocking JSON-RPC client for the Soroban RPC endpoint.
pub struct RpcClient {
    client: Client,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimulateResult {
    /// Minimum resource fee in stroops.
    pub min_resource_fee: Option<String>,
    /// Error message if simulation failed.
    pub error: Option<String>,
    /// Raw simulation response.
    pub raw: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendResult {
    /// Transaction hash.
    pub hash: String,
    /// Submission status ("PENDING", "DUPLICATE", "TRY_AGAIN_LATER", "ERROR").
    pub status: String,
    /// Error result XDR if status is "ERROR".
    pub error_result_xdr: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetTransactionResult {
    /// Final status: "SUCCESS", "FAILED", or "NOT_FOUND".
    pub status: String,
    /// Result XDR (base64) on success.
    pub result_xdr: Option<String>,
    /// Result meta XDR (base64) on success.
    pub result_meta_xdr: Option<String>,
}

impl RpcClient {
    /// Create a new client targeting the given Soroban RPC URL.
    pub fn new(url: &str) -> Self {
        Self {
            client: Client::new(),
            url: url.to_owned(),
        }
    }

    /// Call `simulateTransaction` with a base64-encoded transaction envelope XDR.
    pub fn simulate_transaction(&self, tx_xdr: &str) -> Result<SimulateResult> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": { "transaction": tx_xdr }
        });
        let resp = self.call(body)?;
        let result = resp["result"].clone();
        Ok(SimulateResult {
            min_resource_fee: result["minResourceFee"]
                .as_str()
                .map(str::to_owned),
            error: result["error"].as_str().map(str::to_owned),
            raw: result,
        })
    }

    /// Call `sendTransaction` with a base64-encoded signed transaction envelope XDR.
    pub fn send_transaction(&self, tx_xdr: &str) -> Result<SendResult> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": { "transaction": tx_xdr }
        });
        let resp = self.call(body)?;
        let result = &resp["result"];
        let hash = result["hash"]
            .as_str()
            .context("missing hash in sendTransaction response")?
            .to_owned();
        let status = result["status"]
            .as_str()
            .context("missing status in sendTransaction response")?
            .to_owned();
        Ok(SendResult {
            hash,
            status,
            error_result_xdr: result["errorResultXdr"].as_str().map(str::to_owned),
        })
    }

    /// Call `getTransaction` for the given transaction hash.
    pub fn get_transaction(&self, hash: &str) -> Result<GetTransactionResult> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTransaction",
            "params": { "hash": hash }
        });
        let resp = self.call(body)?;
        let result = &resp["result"];
        let status = result["status"]
            .as_str()
            .context("missing status in getTransaction response")?
            .to_owned();
        Ok(GetTransactionResult {
            status,
            result_xdr: result["resultXdr"].as_str().map(str::to_owned),
            result_meta_xdr: result["resultMetaXdr"].as_str().map(str::to_owned),
        })
    }

    /// Poll `getTransaction` until the transaction is no longer `NOT_FOUND`,
    /// retrying up to `max_attempts` times with `delay_ms` milliseconds between
    /// each attempt.
    pub fn poll_transaction(
        &self,
        hash: &str,
        max_attempts: u32,
        delay_ms: u64,
    ) -> Result<GetTransactionResult> {
        for attempt in 0..max_attempts {
            let result = self.get_transaction(hash)?;
            if result.status != "NOT_FOUND" {
                return Ok(result);
            }
            if attempt + 1 < max_attempts {
                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
        bail!("transaction {hash} not found after {max_attempts} attempts")
    }

    /// Send a raw JSON-RPC request and return the parsed response.
    fn call(&self, body: Value) -> Result<Value> {
        let resp = self
            .client
            .post(&self.url)
            .json(&body)
            .send()
            .context("RPC request failed")?;

        if !resp.status().is_success() {
            bail!("RPC HTTP error: {}", resp.status());
        }

        let json: Value = resp.json().context("failed to parse RPC response")?;

        if let Some(err) = json.get("error") {
            bail!("RPC error: {err}");
        }

        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_client_new() {
        let client = RpcClient::new("https://soroban-testnet.stellar.org");
        assert_eq!(client.url, "https://soroban-testnet.stellar.org");
    }

    /// Verify that a network error (bad URL) returns an error rather than panicking.
    #[test]
    fn test_simulate_bad_url_returns_error() {
        let client = RpcClient::new("http://127.0.0.1:1"); // nothing listening
        let result = client.simulate_transaction("AAAA");
        assert!(result.is_err(), "should fail with connection refused");
    }

    #[test]
    fn test_send_bad_url_returns_error() {
        let client = RpcClient::new("http://127.0.0.1:1");
        let result = client.send_transaction("AAAA");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_transaction_bad_url_returns_error() {
        let client = RpcClient::new("http://127.0.0.1:1");
        let result = client.get_transaction("deadbeef");
        assert!(result.is_err());
    }

    #[test]
    fn test_poll_transaction_bad_url_returns_error() {
        let client = RpcClient::new("http://127.0.0.1:1");
        let result = client.poll_transaction("deadbeef", 1, 0);
        assert!(result.is_err());
    }
}
