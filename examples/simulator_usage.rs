use commonware_avs_node::simulator::{ContractSimulator, SimulationRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mut simulator = ContractSimulator::new();
    
    simulator.start_anvil(
        Some("https://eth-mainnet.g.alchemy.com/v2/your-api-key".to_string()),
        Some(18000000),
    ).await?;

    let request = SimulationRequest {
        target_contract: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        function_selector: "0x70a08231".to_string(),
        function_params: {
            let address = "000000000000000000000000d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
            hex::decode(address)?
        },
        fork_url: None,
        block_number: None,
        sender: None,
        value: None,
    };

    let result = simulator.simulate_transaction(request.clone()).await?;
    
    println!("Simulation Result:");
    println!("  Success: {}", result.success);
    println!("  Gas Used: {}", result.gas_used);
    if !result.return_data.is_empty() {
        println!("  Return Data: 0x{}", hex::encode(&result.return_data));
    }
    if let Some(error) = &result.error_message {
        println!("  Error: {}", error);
    }

    simulator.stop_anvil();
    
    Ok(())
}

fn hex_decode(s: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let s = s.trim_start_matches("0x");
    
    if s.len() % 2 != 0 {
        return Err("Hex string must have even length".into());
    }

    Ok((0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect::<Result<Vec<u8>, _>>()?)
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}