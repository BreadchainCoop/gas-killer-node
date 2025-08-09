use alloy::{
    network::EthereumWallet,
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, Bytes, U256},
    providers::{Provider, ProviderBuilder},
    rpc::types::{TransactionRequest, TransactionReceipt},
    signers::local::PrivateKeySigner,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub target_contract: String,
    pub function_selector: String,
    pub function_params: Vec<u8>,
    pub fork_url: Option<String>,
    pub block_number: Option<u64>,
    pub sender: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub logs: Vec<String>,
    pub error_message: Option<String>,
}

pub struct ContractSimulator {
    anvil: Option<AnvilInstance>,
    rpc_url: String,
}

impl ContractSimulator {
    pub fn new() -> Self {
        Self {
            anvil: None,
            rpc_url: String::new(),
        }
    }

    pub async fn start_anvil(&mut self, fork_url: Option<String>, block_number: Option<u64>) -> Result<()> {
        info!("Starting Anvil instance...");
        
        let mut anvil_builder = Anvil::new();
        
        if let Some(url) = fork_url {
            info!("Forking from: {}", url);
            anvil_builder = anvil_builder.fork(url);
            
            if let Some(block) = block_number {
                info!("Forking at block: {}", block);
                anvil_builder = anvil_builder.fork_block_number(block);
            }
        }
        
        let anvil = anvil_builder.spawn();
        self.rpc_url = anvil.endpoint();
        info!("Anvil running at: {}", self.rpc_url);
        
        self.anvil = Some(anvil);
        Ok(())
    }

    pub async fn simulate_transaction(&self, request: SimulationRequest) -> Result<SimulationResult> {
        info!("Simulating transaction to contract: {}", request.target_contract);
        debug!("Function selector: {}", request.function_selector);
        debug!("Parameters length: {} bytes", request.function_params.len());

        let provider = ProviderBuilder::new()
            .on_http(self.rpc_url.parse()?)
            .await?;

        let target_address = Address::from_str(&request.target_contract)
            .context("Invalid target contract address")?;

        let selector = hex::decode(request.function_selector.trim_start_matches("0x"))
            .context("Invalid function selector")?;
        
        if selector.len() != 4 {
            return Err(anyhow::anyhow!("Function selector must be 4 bytes"));
        }

        let mut calldata = selector;
        calldata.extend_from_slice(&request.function_params);
        
        let sender = if let Some(sender_str) = request.sender {
            Address::from_str(&sender_str).context("Invalid sender address")?
        } else {
            let accounts = provider.get_accounts().await?;
            accounts.first()
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No accounts available"))?
        };

        let value = if let Some(value_str) = request.value {
            U256::from_str(&value_str).context("Invalid value")?
        } else {
            U256::ZERO
        };

        let tx_request = TransactionRequest::default()
            .from(sender)
            .to(target_address)
            .value(value)
            .input(Bytes::from(calldata));

        match provider.call(&tx_request).await {
            Ok(result) => {
                info!("Simulation successful");
                
                let gas_estimate = provider
                    .estimate_gas(&tx_request)
                    .await
                    .unwrap_or(21000);

                Ok(SimulationResult {
                    success: true,
                    return_data: result.to_vec(),
                    gas_used: gas_estimate,
                    logs: Vec::new(),
                    error_message: None,
                })
            }
            Err(e) => {
                info!("Simulation failed: {}", e);
                Ok(SimulationResult {
                    success: false,
                    return_data: Vec::new(),
                    gas_used: 0,
                    logs: Vec::new(),
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    pub async fn simulate_and_execute(&self, request: SimulationRequest) -> Result<SimulationResult> {
        info!("Simulating and executing transaction...");
        
        let provider = ProviderBuilder::new()
            .on_http(self.rpc_url.parse()?)
            .await?;

        let target_address = Address::from_str(&request.target_contract)
            .context("Invalid target contract address")?;

        let selector = hex::decode(request.function_selector.trim_start_matches("0x"))
            .context("Invalid function selector")?;
        
        if selector.len() != 4 {
            return Err(anyhow::anyhow!("Function selector must be 4 bytes"));
        }

        let mut calldata = selector;
        calldata.extend_from_slice(&request.function_params);

        let accounts = provider.get_accounts().await?;
        let sender = if let Some(sender_str) = request.sender {
            Address::from_str(&sender_str).context("Invalid sender address")?
        } else {
            accounts.first()
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No accounts available"))?
        };

        let value = if let Some(value_str) = request.value {
            U256::from_str(&value_str).context("Invalid value")?
        } else {
            U256::ZERO
        };

        let private_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let signer = PrivateKeySigner::from_str(private_key)?;
        let wallet = EthereumWallet::from(signer);
        
        let provider_with_wallet = ProviderBuilder::new()
            .wallet(wallet)
            .on_http(self.rpc_url.parse()?)
            .await?;

        let tx_request = TransactionRequest::default()
            .from(sender)
            .to(target_address)
            .value(value)
            .input(Bytes::from(calldata));

        match provider_with_wallet.send_transaction(tx_request).await {
            Ok(pending_tx) => {
                info!("Transaction sent, waiting for receipt...");
                
                let receipt = pending_tx
                    .get_receipt()
                    .await
                    .context("Failed to get transaction receipt")?;

                let logs: Vec<String> = receipt
                    .inner
                    .logs()
                    .iter()
                    .map(|log| format!("{:?}", log))
                    .collect();

                let gas_used = receipt.gas_used;
                
                Ok(SimulationResult {
                    success: receipt.status(),
                    return_data: Vec::new(),
                    gas_used,
                    logs,
                    error_message: None,
                })
            }
            Err(e) => {
                info!("Transaction failed: {}", e);
                Ok(SimulationResult {
                    success: false,
                    return_data: Vec::new(),
                    gas_used: 0,
                    logs: Vec::new(),
                    error_message: Some(e.to_string()),
                })
            }
        }
    }

    pub fn stop_anvil(&mut self) {
        if let Some(anvil) = self.anvil.take() {
            info!("Stopping Anvil instance");
            drop(anvil);
        }
    }
}

impl Drop for ContractSimulator {
    fn drop(&mut self) {
        self.stop_anvil();
    }
}

pub mod hex {
    use anyhow::Result;

    pub fn decode(s: &str) -> Result<Vec<u8>> {
        let s = s.trim_start_matches("0x");
        
        if s.len() % 2 != 0 {
            return Err(anyhow::anyhow!("Hex string must have even length"));
        }

        (0..s.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&s[i..i + 2], 16)
                    .map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))
            })
            .collect()
    }

    pub fn encode(bytes: &[u8]) -> String {
        format!("0x{}", bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_anvil_startup() {
        let mut simulator = ContractSimulator::new();
        assert!(simulator.start_anvil(None, None).await.is_ok());
        assert!(!simulator.rpc_url.is_empty());
        simulator.stop_anvil();
    }

    #[tokio::test]
    async fn test_hex_decode() {
        assert_eq!(hex::decode("0x12345678").unwrap(), vec![0x12, 0x34, 0x56, 0x78]);
        assert_eq!(hex::decode("12345678").unwrap(), vec![0x12, 0x34, 0x56, 0x78]);
        assert!(hex::decode("0x123").is_err());
    }

    #[tokio::test]
    async fn test_hex_encode() {
        assert_eq!(hex::encode(&[0x12, 0x34, 0x56, 0x78]), "0x12345678");
    }
}