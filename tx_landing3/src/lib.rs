use base64::{Engine as _, engine::general_purpose::STANDARD};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_account_decoder::UiAccountEncoding;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;
use zela_std::RpcError;
use zela_std::rpc_client::{
    RpcSendTransactionConfig, RpcSimulateTransactionAccountsConfig, RpcSimulateTransactionConfig,
};
use zela_std::{CustomProcedure, JsonValue, rpc_client::RpcClient, zela_custom_procedure};

pub struct SendTransaction;

#[derive(Deserialize, Debug)]
pub struct TxVariant {
    pub tx: String,
    pub fee_lamports: u64,
}

#[derive(Deserialize, Debug)]
pub struct Input {
    pub variants: Vec<TxVariant>,
    pub watch_account: String,
    pub is_sol: bool,
    pub price_in_sol: f64,
    pub simulate_retries: Option<u32>,
}

#[derive(Serialize, Clone)]
pub struct SimulateResult {
    pub success: bool,
    pub logs: Vec<String>,
    pub units_consumed: Option<u64>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct Output {
    pub time_elapsed: i64,
    pub simulation: Option<SimulateResult>,
    pub simulation_attempts: u32,
    pub best_fee_lamports: Option<u64>,
    pub signature: Option<String>,
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
        let mut total_attempts = 0u32;

        let first = params.variants.first().ok_or_else(|| RpcError {
            code: 400,
            message: "no variants provided".into(),
            data: None,
        })?;

        let tx_bytes = STANDARD.decode(&first.tx).map_err(|e| RpcError {
            code: 400,
            message: format!("base64 decode error: {}", e),
            data: None,
        })?;
        let tx: VersionedTransaction = bincode::deserialize(&tx_bytes).map_err(|e| RpcError {
            code: 400,
            message: format!("tx deserialize error: {}", e),
            data: None,
        })?;
        let watch_pubkey: Pubkey = params.watch_account.parse().map_err(|e| RpcError {
            code: 400,
            message: format!("invalid pubkey: {}", e),
            data: None,
        })?;

        let config = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            commitment: Some(CommitmentConfig::processed()),
            accounts: Some(RpcSimulateTransactionAccountsConfig {
                encoding: Some(if params.is_sol {
                    UiAccountEncoding::Base64
                } else {
                    UiAccountEncoding::JsonParsed
                }),
                addresses: vec![params.watch_account.clone()],
            }),
            ..Default::default()
        };

        let mut last_sim: Option<SimulateResult> = None;
        let mut best_fee_lamports: Option<u64> = None;
        let mut best_tx_b64: Option<String> = None;

        for _ in 0..max_retries {
            total_attempts += 1;
            let sim_result = client
                .simulate_transaction_with_config(&tx, config.clone())
                .await?
                .value;

            let sim = SimulateResult {
                success: sim_result.err.is_none(),
                logs: sim_result.logs.unwrap_or_default(),
                units_consumed: sim_result.units_consumed,
                error: sim_result.err.map(|e| format!("{:?}", e)),
            };

            if sim.success {
                // get balance before
                // fetch pre balance
                let pre_balance: Option<u64> = if params.is_sol {
                    client.get_balance(&watch_pubkey).await.ok()
                } else {
                    client
                        .get_token_account_balance(&watch_pubkey)
                        .await
                        .ok()
                        .and_then(|r| r.amount.parse::<u64>().ok())
                };
                // extract post balance
                let post_balance: Option<u64> = sim_result
                    .accounts
                    .and_then(|accs| accs.into_iter().next().flatten())
                    .and_then(|acc| {
                        if params.is_sol {
                            Some(acc.lamports)
                        } else {
                            extract_token_amount(&acc)
                        }
                    });

                // compute profit and pick best variant
                if let (Some(pre), Some(post)) = (pre_balance, post_balance) {
                    let delta = post as i64 - pre as i64;
                    let profit = (delta as f64 * params.price_in_sol) as i64;

                    let best = params
                        .variants
                        .iter()
                        .filter(|v| profit > v.fee_lamports as i64)
                        .max_by_key(|v| v.fee_lamports);

                    if let Some(v) = best {
                        best_fee_lamports = Some(v.fee_lamports);
                        best_tx_b64 = Some(v.tx.clone());
                    }
                }
                last_sim = Some(sim);
                break;
            }
            last_sim = Some(sim);
        }

        // send best tx without simulation
        let signature = if let Some(b64) = best_tx_b64 {
            let tx_bytes = STANDARD.decode(&b64).map_err(|e| RpcError {
                code: 400,
                message: format!("base64 decode error: {}", e),
                data: None,
            })?;
            let best_tx: VersionedTransaction =
                bincode::deserialize(&tx_bytes).map_err(|e| RpcError {
                    code: 400,
                    message: format!("tx deserialize error: {}", e),
                    data: None,
                })?;
            client
                .send_transaction_with_config(
                    &best_tx,
                    RpcSendTransactionConfig {
                        skip_preflight: true,
                        ..Default::default()
                    },
                )
                .await
                .ok()
                .map(|s| s.to_string())
        } else {
            None
        };

        let elapsed = (Utc::now() - start).num_microseconds().unwrap_or_default();

        Ok(Output {
            time_elapsed: elapsed,
            simulation: last_sim,
            simulation_attempts: total_attempts,
            best_fee_lamports,
            signature,
        })
    }
}

fn extract_token_amount(acc: &solana_account_decoder::UiAccount) -> Option<u64> {
    if let solana_account_decoder::UiAccountData::Json(parsed) = &acc.data {
        parsed
            .parsed
            .get("info")?
            .get("tokenAmount")?
            .get("amount")?
            .as_str()?
            .parse::<u64>()
            .ok()
    } else {
        None
    }
}

zela_custom_procedure!(SendTransaction);
