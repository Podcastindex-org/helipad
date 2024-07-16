use async_trait::async_trait;

use std::collections::HashMap;
use std::error::Error;

use crate::HelipadConfig;

use crate::lnclient::clnclient::CLNClient;
use crate::lnclient::lndclient::LNDClient;

pub mod lndclient;
pub mod clnclient;

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub pubkey: String,
    pub alias: String,
    pub nodetype: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Boost {
    pub action: u8,
    pub podcast: String,
    pub episode: String,
    pub message: String,
    pub app: String,
    pub remote_feed_guid: String,
    pub remote_item_guid: String,
    pub sender: String,
    pub tlv: String,
    pub value_msat: i64,
    pub value_msat_total: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub index: u64,
    pub time: i64,
    pub amount: i64,

    pub payment_hash: String,
    pub preimage: String,

    pub boostagram: Option<Boost>,
    pub custom_records: HashMap<u64, Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payment {
    pub index: u64,
    pub time: i64,
    pub amount: i64,

    pub payment_hash: String,
    pub payment_preimage: String,

    pub boostagram: Option<Boost>,
    pub custom_records: HashMap<u64, Vec<u8>>,

    pub destination: String,
    pub fee: i64,
}

#[async_trait]
pub trait LNClient: Send {
    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error>>;
    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error>>;
    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error>>;
    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error>>;
    async fn keysend(&mut self, destination: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error>>;
}

pub async fn connect(config: &HelipadConfig) -> Result<Box<dyn LNClient>, Box<dyn Error>> {
    let helipad_config = config.clone();

    if helipad_config.cln_url != "" {
        Ok(Box::new(CLNClient::connect(helipad_config.cln_url, helipad_config.cln_cert_path, helipad_config.cln_key_path, helipad_config.cln_cacert_path).await?))
    }
    else {
        Ok(Box::new(LNDClient::connect(helipad_config.lnd_url, helipad_config.lnd_cert_path, helipad_config.lnd_macaroon_path).await?))
    }
}
