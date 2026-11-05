use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::pin::Pin;

pub mod cln_client;
pub mod lnd_client;

pub use cln_client::CLNClient;
pub use lnd_client::LNDClient;

pub mod proto;

#[derive(Debug)]
pub struct LNClientError(pub String);

impl fmt::Display for LNClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for LNClientError {}

#[derive(Clone, Debug)]
pub struct NodeInfo {
    pub pubkey: String,
    pub alias: String,
    pub nodetype: String,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub index: u64,
    pub time: i64,
    pub amount: i64,
    pub memo: String,

    pub payment_hash: String,
    pub preimage: String,

    pub custom_records: HashMap<u64, Vec<u8>>,
    pub settled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Payment {
    pub index: u64,
    pub time: i64,
    pub amount: i64,

    pub payment_hash: String,
    pub payment_preimage: String,

    pub custom_records: HashMap<u64, Vec<u8>>,

    pub destination: String,
    pub fee: i64,
}

pub type InvoiceStream = Pin<Box<dyn Stream<Item = Result<Invoice, LNClientError>> + Send>>;

#[async_trait]
pub trait LNClient: Send {
    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error + Send + Sync>>;
    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error + Send + Sync>>;
    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error + Send + Sync>>;
    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error + Send + Sync>>;
    /// Invoices after `start_index`. LND resumes on add index. CLN resumes on pay index. `0` starts at the beginning.
    async fn subscribe_invoices(&mut self, start_index: u64) -> Result<InvoiceStream, Box<dyn Error + Send + Sync>>;
    async fn send_keysend(&mut self, dest: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>>;
    async fn send_bolt11(&mut self, payment_request: String, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>>;
}

pub async fn connect_lnd(url: String, cert_path: String, macaroon_path: String) -> Result<LNDClient, Box<dyn Error + Send + Sync>> {
    lnd_client::connect(url, cert_path, macaroon_path).await
}

pub async fn connect_cln(url: String, cert_path: String, key_path: String, cacert_path: String) -> Result<CLNClient, Box<dyn Error + Send + Sync>> {
    cln_client::connect(url, cert_path, key_path, cacert_path).await
}