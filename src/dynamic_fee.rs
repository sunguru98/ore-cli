use crate::Miner;

use ore_api::consts::BUS_ADDRESSES;
use reqwest::Client;
use serde_json::{json, Value};

impl Miner {
    pub async fn dynamic_fee(&self, difficulty: Option<u32>) -> u64 {
        let ore_addresses: Vec<String> =
            std::iter::once("oreV2ZymfyeXgNgBdqMkumTqqAprVqgBWQfoYkrtKWQ".to_string())
                .chain(BUS_ADDRESSES.iter().map(|pubkey| pubkey.to_string()))
                .collect();

        let priority_level = if let Some(difficulty) = difficulty {
            if difficulty < 10 {
                "Low"
            } else if difficulty > 10 && difficulty < 15 {
                "Medium"
            } else if difficulty > 15 && difficulty < 18 {
                "High"
            } else {
                "VeryHigh"
            }
        } else {
            "Medium"
        };

        match &self.dynamic_fee_strategy {
            None => self.priority_fee.unwrap_or(0),
            Some(strategy) => {
                let client = Client::new();

                let body = match strategy.as_str() {
                    "helius" => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": "priority-fee-estimate",
                            "method": "getPriorityFeeEstimate",
                            "params": [{
                                "accountKeys": ore_addresses,
                                "options": {
                                    "priorityLevel": priority_level
                                }
                            }]
                        })
                    }
                    "triton" => {
                        json!({
                            "jsonrpc": "2.0",
                            "id": "priority-fee-estimate",
                            "method": "getRecentPrioritizationFees",
                            "params": [
                                ore_addresses,
                                {
                                    "percentile": 5000,
                                }
                            ]
                        })
                    }
                    _ => return self.priority_fee.unwrap_or(0),
                };

                let response: Value = client
                    .post(self.dynamic_fee_url.as_ref().unwrap())
                    .json(&body)
                    .send()
                    .await
                    .unwrap()
                    .json()
                    .await
                    .unwrap();

                match strategy.as_str() {
                    "helius" => response["result"]["priorityFeeEstimate"]
                        .as_f64()
                        .map(|fee| fee as u64)
                        .ok_or_else(|| {
                            format!("Failed to parse priority fee. Response: {:?}", response)
                        })
                        .unwrap(),
                    "triton" => response["result"]
                        .as_array()
                        .and_then(|arr| arr.last())
                        .and_then(|last| last["prioritizationFee"].as_u64())
                        .ok_or_else(|| {
                            format!("Failed to parse priority fee. Response: {:?}", response)
                        })
                        .unwrap(),
                    _ => return self.priority_fee.unwrap_or(0),
                }
            }
        }
    }
}
