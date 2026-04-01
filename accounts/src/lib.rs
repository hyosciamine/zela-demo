use std::str::FromStr;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentConfig;
use zela_std::{CustomProcedure, JsonValue, rpc_client::RpcClient, zela_custom_procedure};
use solana_sdk::pubkey::Pubkey;
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[derive(Serialize)]
pub struct Output {
    pub time_elapsed: i64,
    #[serde(serialize_with = "as_base64")]
    pub data: Vec<u8>,
}

fn as_base64<S: serde::Serializer>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&STANDARD.encode(bytes))
}

// Define an empty struct to serve as a binding to rockrpc_custom_procedure trait.
pub struct Accounts;

#[derive(Deserialize, Debug)]
pub struct Input {
    account: String,
}

// // Define output data of your method
// #[derive(Serialize)]
// pub struct Output {
//     pub time_elapsed: i64,
//     pub data: Vec<u8>,
// }

impl CustomProcedure for Accounts {
    // We do not need any params for thisA procedure
    type Params = Input;
    type SuccessData = Output;
    type ErrorData = JsonValue;

    // Run method is the entry point of every custom procedure.
    // It will be called once for each incoming request.
    async fn run(
        params : Self::Params,
    ) -> Result<Self::SuccessData, zela_std::RpcError<Self::ErrorData>> {
        let account = params.account;
        let pubkey = Pubkey::from_str(&account).unwrap();
        let client = RpcClient::new_with_commitment(CommitmentConfig::processed());
        let start = Utc::now();

        let data = client.get_account_data(&pubkey).await?;
        let end = Utc::now();

        let time_elapsed = (end - start).num_microseconds().unwrap_or_default();

        // Assemble response struct.
        // It will be serialized into the JSON response using serde_json.
        let response = Output {
            time_elapsed,
            data,
        };

        Ok(response)
    }
}

// This is an essential macro-call that enables us to run a procedure
zela_custom_procedure!(Accounts);
