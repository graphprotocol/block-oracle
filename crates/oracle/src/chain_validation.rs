use crate::config::{IndexedChain, ProtocolChain};
use crate::runner::jrpc_utils::JrpcExpBackoff;
use crate::{Caip2ChainId, Config};
use anyhow::anyhow;
use std::str::FromStr;
use tracing::{error, info};
use web3::api::Web3;
use web3::helpers::CallFuture;

/// Validates that all configured RPC endpoints return the expected chain ID
pub async fn validate_chain_ids(config: &Config) -> anyhow::Result<()> {
    info!("Validating RPC chain IDs...");

    // Validate protocol chain
    validate_protocol_chain(&config.protocol_chain).await?;

    // Validate indexed chains
    for chain in &config.indexed_chains {
        validate_indexed_chain(chain).await?;
    }

    info!("All RPC chain IDs validated successfully");
    Ok(())
}

async fn validate_protocol_chain(chain: &ProtocolChain) -> anyhow::Result<()> {
    let transport = JrpcExpBackoff::http(
        chain.jrpc_url.clone(),
        chain.id.clone(),
        std::time::Duration::from_secs(30),
    );
    let web3 = Web3::new(transport);

    validate_chain_id(&web3, &chain.id, chain.jrpc_url.as_ref()).await
}

async fn validate_indexed_chain(chain: &IndexedChain) -> anyhow::Result<()> {
    let transport = JrpcExpBackoff::http(
        chain.jrpc_url.clone(),
        chain.id.clone(),
        std::time::Duration::from_secs(30),
    );
    let web3 = Web3::new(transport);

    validate_chain_id(&web3, &chain.id, chain.jrpc_url.as_ref()).await
}

async fn validate_chain_id<T>(
    web3: &Web3<T>,
    expected_chain: &Caip2ChainId,
    rpc_url: &str,
) -> anyhow::Result<()>
where
    T: web3::Transport,
{
    // Only validate EVM chains (namespace "eip155")
    if expected_chain.namespace_part() != "eip155" {
        info!(
            "Skipping chain ID validation for non-EVM chain: {}",
            expected_chain
        );
        return Ok(());
    }

    // Parse expected numeric chain ID from CAIP-2 reference
    let expected_numeric_id = u64::from_str(expected_chain.reference_part())
        .map_err(|e| anyhow!("Failed to parse chain ID from {}: {}", expected_chain, e))?;

    // Call eth_chainId
    let fut = web3.transport().execute("eth_chainId", vec![]);
    let call_fut: CallFuture<String, T::Out> = CallFuture::new(fut);

    let chain_id_hex = match call_fut.await {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to get chain ID from RPC {}: {}", rpc_url, e);
            return Err(anyhow!(
                "Failed to get chain ID from RPC {}: {}",
                rpc_url,
                e
            ));
        }
    };

    // Parse hex chain ID (e.g., "0xa4b1" -> 42161)
    let actual_chain_id = u64::from_str_radix(chain_id_hex.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow!("Failed to parse chain ID hex '{}': {}", chain_id_hex, e))?;

    // Compare
    if actual_chain_id != expected_numeric_id {
        error!(
            "Chain ID mismatch for {}: RPC {} returned chain ID {} (0x{:x}), expected {} from CAIP-2 identifier {}",
            expected_chain, rpc_url, actual_chain_id, actual_chain_id, expected_numeric_id, expected_chain
        );
        return Err(anyhow!(
            "Chain ID mismatch for {}: RPC {} returned chain ID {} (0x{:x}), expected {} from CAIP-2 identifier {}",
            expected_chain, rpc_url, actual_chain_id, actual_chain_id, expected_numeric_id, expected_chain
        ));
    }

    info!(
        "✓ Chain ID validated for {}: RPC {} correctly returns chain ID {}",
        expected_chain, rpc_url, actual_chain_id
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpc_core::{Call, Value};
    use std::future::Future;
    use std::pin::Pin;
    use web3::RequestId;

    #[test]
    fn test_caip2_parsing() {
        let chain_id = Caip2ChainId::from_str("eip155:1").unwrap();
        assert_eq!(chain_id.namespace_part(), "eip155");
        assert_eq!(chain_id.reference_part(), "1");

        let chain_id = Caip2ChainId::from_str("eip155:42161").unwrap();
        assert_eq!(chain_id.namespace_part(), "eip155");
        assert_eq!(chain_id.reference_part(), "42161");

        // Non-EVM chain
        let chain_id = Caip2ChainId::from_str("bip122:000000000019d6689c085ae165831e93").unwrap();
        assert_eq!(chain_id.namespace_part(), "bip122");
        assert_eq!(
            chain_id.reference_part(),
            "000000000019d6689c085ae165831e93"
        );
    }

    // Mock transport that returns a predefined chain ID
    #[derive(Debug, Clone)]
    struct MockTransport {
        chain_id_response: String,
    }

    impl web3::Transport for MockTransport {
        type Out = Pin<Box<dyn Future<Output = Result<Value, web3::Error>>>>;

        fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
            let call = Call::MethodCall(jsonrpc_core::MethodCall {
                jsonrpc: Some(jsonrpc_core::Version::V2),
                method: method.to_string(),
                params: jsonrpc_core::Params::Array(params),
                id: jsonrpc_core::Id::Num(1),
            });
            (1, call)
        }

        fn send(&self, _id: RequestId, request: Call) -> Self::Out {
            let response = match request {
                Call::MethodCall(ref call) if call.method == "eth_chainId" => {
                    Ok(Value::String(self.chain_id_response.clone()))
                }
                Call::MethodCall(ref call) => Err(web3::Error::Decoder(format!(
                    "Unexpected method: {}",
                    call.method
                ))),
                _ => Err(web3::Error::Decoder("Invalid request type".to_string())),
            };

            Box::pin(futures::future::ready(response))
        }
    }

    #[tokio::test]
    async fn test_validate_chain_id_success() {
        let mock_transport = MockTransport {
            chain_id_response: "0x1".to_string(),
        };
        let web3 = Web3::new(mock_transport);
        let chain_id = Caip2ChainId::from_str("eip155:1").unwrap();

        let result = validate_chain_id(&web3, &chain_id, "http://test.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_chain_id_mismatch() {
        let mock_transport = MockTransport {
            chain_id_response: "0x1".to_string(), // Returns mainnet (1)
        };
        let web3 = Web3::new(mock_transport);
        let chain_id = Caip2ChainId::from_str("eip155:42161").unwrap(); // Expects Arbitrum (42161)

        let result = validate_chain_id(&web3, &chain_id, "http://test.com").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Chain ID mismatch"));
        assert!(err_msg.contains("returned chain ID 1"));
        assert!(err_msg.contains("expected 42161"));
    }

    #[tokio::test]
    async fn test_validate_chain_id_hex_variations() {
        // Test with different hex formats
        let test_cases = vec![
            ("0x1", 1),             // 0x1
            ("0x01", 1),            // 0x01
            ("0xa4b1", 42161),      // 0xa4b1 (Arbitrum)
            ("0xaa36a7", 11155111), // Sepolia
        ];

        for (hex_response, expected_id) in test_cases {
            let mock_transport = MockTransport {
                chain_id_response: hex_response.to_string(),
            };
            let web3 = Web3::new(mock_transport);
            let chain_id = Caip2ChainId::from_str(&format!("eip155:{}", expected_id)).unwrap();

            let result = validate_chain_id(&web3, &chain_id, "http://test.com").await;
            assert!(
                result.is_ok(),
                "Failed for hex {} expecting {}",
                hex_response,
                expected_id
            );
        }
    }

    #[tokio::test]
    async fn test_validate_chain_id_skips_non_evm() {
        // Non-EVM chains should be skipped
        let mock_transport = MockTransport {
            chain_id_response: "should_not_be_called".to_string(),
        };
        let web3 = Web3::new(mock_transport);
        let chain_id = Caip2ChainId::from_str("bip122:000000000019d6689c085ae165831e93").unwrap();

        let result = validate_chain_id(&web3, &chain_id, "http://test.com").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_chain_id_invalid_hex() {
        let mock_transport = MockTransport {
            chain_id_response: "invalid_hex".to_string(),
        };
        let web3 = Web3::new(mock_transport);
        let chain_id = Caip2ChainId::from_str("eip155:1").unwrap();

        let result = validate_chain_id(&web3, &chain_id, "http://test.com").await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse chain ID hex"));
    }
}
