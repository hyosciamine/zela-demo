use std::str::FromStr;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::transaction::VersionedTransaction;
use zela_std::{CustomProcedure, JsonValue, rpc_client::RpcClient, zela_custom_procedure};
use zela_std::RpcError;
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub struct SendTransaction;

#[derive(Deserialize, Debug)]
pub struct Input {
    pub tx: String,
    pub simulate: bool,
    pub simulate_retries: Option<u32>, // how many times to retry simulation, default 10
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
    pub simulation_attempts: u32,
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
        let max_retries = params.simulate_retries.unwrap_or(10);

        // decode base64 tx
        let tx_bytes = STANDARD.decode(&params.tx).map_err(|e| RpcError {
            code: 400,
            message: format!("base64 decode error: {}", e),
            data: None,
        })?;

        let tx: VersionedTransaction = bincode::deserialize(&tx_bytes).map_err(|e| RpcError {
            code: 400,
            message: format!("tx deserialize error: {}", e),
            data: None,
        })?;

        // simulate in loop if requested
        let simulation = if params.simulate {
            let mut last_sim: Option<SimulateResult> = None;
            let mut attempts = 0u32;
            let mut passed = false;

            for _ in 0..max_retries {
                attempts += 1;
                let sim_result = client.simulate_transaction(&tx).await?.value;
                let sim = SimulateResult {
                    success: sim_result.err.is_none(),
                    logs: sim_result.logs.unwrap_or_default(),
                    units_consumed: sim_result.units_consumed,
                    error: sim_result.err.map(|e| format!("{:?}", e)),
                };

                if sim.success {
                    passed = true;
                    last_sim = Some(sim);
                    break;
                }

                last_sim = Some(sim);
            }

            // all retries failed — return last simulation result
            if !passed {
                let elapsed = (Utc::now() - start).num_microseconds().unwrap_or_default();
                return Ok(Output {
                    time_elapsed: elapsed,
                    signature: None,
                    simulation: last_sim,
                    simulation_attempts: attempts,
                });
            }

            last_sim
        } else {
            None
        };

        // send transaction
        let signature = client.send_transaction(&tx).await?;
        let elapsed = (Utc::now() - start).num_milliseconds();

        Ok(Output {
            time_elapsed: elapsed,
            signature: Some(signature.to_string()),
            simulation,
            simulation_attempts: if params.simulate { max_retries } else { 0 },
        })
    }
}

zela_custom_procedure!(SendTransaction);