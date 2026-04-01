use std::str::FromStr;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::transaction::Transaction;
use zela_std::{CustomProcedure, JsonValue, rpc_client::RpcClient, zela_custom_procedure};
use zela_std::RpcError;
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub struct SendTransaction;

#[derive(Deserialize, Debug)]
pub struct Input {
    pub tx: String,       // base64 encoded transaction
    pub simulate: bool,   // if true, simulate before sending
}

#[derive(Serialize)]
pub struct SimulateResult {
    pub success: bool,
    pub logs: Vec<String>,
    pub units_consumed: Option<u64>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Output {
    pub time_elapsed: i64,
    pub signature: Option<String>,
    pub simulation: Option<SimulateResult>,
}

impl CustomProcedure for SendTransaction {
    type Params = Input;
    type SuccessData = Output;
    type ErrorData = JsonValue;

    async fn run(
        params: Self::Params,
    ) -> Result<Self::SuccessData, zela_std::RpcError<Self::ErrorData>> {
        let client = RpcClient::new_with_commitment(CommitmentConfig::processed());
        let start = Utc::now();

        // decode base64 tx
        // decode base64 tx
        let tx_bytes = STANDARD.decode(&params.tx).map_err(|e| RpcError {
            code: 400,
            message: format!("base64 decode error: {}", e),
            data: None,
        })?;

        let tx: Transaction = bincode::deserialize(&tx_bytes).map_err(|e| RpcError {
            code: 400,
            message: format!("tx deserialize error: {}", e),
            data: None,
        })?;

        // simulate if requested
        let simulation = if params.simulate {
            let sim_result = client.simulate_transaction(&tx).await?.value;
            let sim = SimulateResult {
                success: sim_result.err.is_none(),
                logs: sim_result.logs.unwrap_or_default(),
                units_consumed: sim_result.units_consumed,
                error: sim_result.err.map(|e| format!("{:?}", e)),
            };

            // if simulation failed, return early without sending
            if !sim.success {
                let elapsed = (Utc::now() - start).num_microseconds().unwrap_or_default();
                return Ok(Output {
                    time_elapsed: elapsed,
                    signature: None,
                    simulation: Some(sim),
                });
            }

            Some(sim)
        } else {
            None
        };

        // send transaction
        let signature = client.send_transaction(&tx).await?;

        let elapsed = (Utc::now() - start).num_microseconds().unwrap_or_default();

        Ok(Output {
            time_elapsed: elapsed,
            signature: Some(signature.to_string()),
            simulation,
        })
    }
}

zela_custom_procedure!(SendTransaction);