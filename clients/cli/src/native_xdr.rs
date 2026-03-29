/// Native XDR encoding/decoding using the `stellar-xdr` crate (issue #144).
///
/// Provides helpers for constructing and encoding Soroban contract invocation
/// arguments as XDR without shelling out to the `stellar` binary.
///
/// The `stellar-xdr` crate's `WriteXdr` / `ReadXdr` traits are used for all
/// serialisation so the output is byte-for-byte identical to what the Stellar
/// network expects.
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use stellar_xdr::curr::{
    Hash, InvokeContractArgs, ScAddress, ScSymbol, ScVal, WriteXdr, ReadXdr, Limits,
};

/// Encode an `ScVal` to raw XDR bytes.
pub fn encode_scval(val: &ScVal) -> Result<Vec<u8>> {
    val.to_xdr(Limits::none()).context("XDR encoding failed")
}

/// Decode raw XDR bytes into an `ScVal`.
pub fn decode_scval(bytes: &[u8]) -> Result<ScVal> {
    ScVal::from_xdr(bytes, Limits::none()).context("XDR decoding failed")
}

/// Encode an `ScVal` to a base64 string (standard alphabet, with padding).
/// This is the format expected by the Soroban RPC endpoint.
pub fn encode_scval_base64(val: &ScVal) -> Result<String> {
    let bytes = encode_scval(val)?;
    Ok(STANDARD.encode(bytes))
}

/// Decode a base64-encoded XDR string into an `ScVal`.
pub fn decode_scval_base64(b64: &str) -> Result<ScVal> {
    let bytes = STANDARD
        .decode(b64.trim())
        .context("invalid base64 in XDR payload")?;
    decode_scval(&bytes)
}

/// Build an `ScVal::I128` from an i128 value.
pub fn scval_i128(value: i128) -> ScVal {
    ScVal::I128(stellar_xdr::curr::Int128Parts {
        hi: (value >> 64) as i64,
        lo: value as u64,
    })
}

/// Build an `ScVal::Symbol` from a short symbol string (≤ 32 chars).
pub fn scval_symbol(s: &str) -> Result<ScVal> {
    let sym: ScSymbol = s
        .try_into()
        .map_err(|_| anyhow::anyhow!("symbol too long (max 32 chars)"))?;
    Ok(ScVal::Symbol(sym))
}

/// Build an `ScVal::U64` from a u64 value.
pub fn scval_u64(value: u64) -> ScVal {
    ScVal::U64(value)
}

/// Build an `ScVal::Void`.
pub fn scval_void() -> ScVal {
    ScVal::Void
}

/// Construct the `InvokeContractArgs` XDR for a contract function call and
/// return it as a base64-encoded string.
///
/// `contract_id_hex` is the 32-byte contract ID as a hex string (64 chars).
pub fn build_invoke_contract_args_xdr(
    contract_id_hex: &str,
    function_name: &str,
    args: Vec<ScVal>,
) -> Result<String> {
    let hash_bytes = hex::decode(contract_id_hex)
        .context("contract_id_hex must be a 64-char hex string")?;
    if hash_bytes.len() != 32 {
        anyhow::bail!("contract ID must be exactly 32 bytes (64 hex chars)");
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&hash_bytes);

    let invoke = InvokeContractArgs {
        contract_address: ScAddress::Contract(Hash(arr)),
        function_name: function_name
            .try_into()
            .map_err(|_| anyhow::anyhow!("function name too long"))?,
        args: args
            .try_into()
            .map_err(|_| anyhow::anyhow!("too many arguments"))?,
    };

    let bytes = invoke
        .to_xdr(Limits::none())
        .context("failed to encode InvokeContractArgs")?;
    Ok(STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scval_void_roundtrip() {
        let val = scval_void();
        let bytes = encode_scval(&val).expect("encode");
        let decoded = decode_scval(&bytes).expect("decode");
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_scval_u64_roundtrip() {
        let val = scval_u64(42_000_000);
        let bytes = encode_scval(&val).expect("encode");
        let decoded = decode_scval(&bytes).expect("decode");
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_scval_i128_roundtrip() {
        let val = scval_i128(1_000_000_000);
        let bytes = encode_scval(&val).expect("encode");
        let decoded = decode_scval(&bytes).expect("decode");
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_scval_i128_negative_roundtrip() {
        let val = scval_i128(-1);
        let bytes = encode_scval(&val).expect("encode");
        let decoded = decode_scval(&bytes).expect("decode");
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_scval_symbol_roundtrip() {
        let val = scval_symbol("approve").expect("build");
        let bytes = encode_scval(&val).expect("encode");
        let decoded = decode_scval(&bytes).expect("decode");
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_encode_base64_roundtrip() {
        let val = scval_u64(999);
        let b64 = encode_scval_base64(&val).expect("encode");
        let decoded = decode_scval_base64(&b64).expect("decode");
        assert_eq!(val, decoded);
    }

    #[test]
    fn test_decode_invalid_base64_returns_error() {
        assert!(decode_scval_base64("not!!valid@@base64").is_err());
    }

    #[test]
    fn test_decode_invalid_xdr_returns_error() {
        // Valid base64 but not valid XDR
        let b64 = STANDARD.encode(b"garbage");
        assert!(decode_scval_base64(&b64).is_err());
    }

    #[test]
    fn test_build_invoke_contract_args_bad_hex() {
        let result = build_invoke_contract_args_xdr("not-hex", "approve", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_invoke_contract_args_wrong_length() {
        let result = build_invoke_contract_args_xdr("deadbeef", "approve", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_invoke_contract_args_valid() {
        let contract_id = "aa".repeat(32); // 32 bytes as hex
        let result = build_invoke_contract_args_xdr(&contract_id, "approve", vec![scval_void()]);
        assert!(result.is_ok(), "should encode successfully: {:?}", result);
        let b64 = result.unwrap();
        assert!(!b64.is_empty());
    }
}
