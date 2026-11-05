use async_trait::async_trait;

use crate::{Invoice, InvoiceStream, LNClient, LNClientError, NodeInfo, Payment};

use data_encoding::HEXLOWER;

use futures::StreamExt;

use lnd::lnrpc::lnrpc::invoice::InvoiceState;
use lnd::lnrpc::lnrpc::{Invoice as LndInvoice, Payment as LndPayment, payment::PaymentStatus};
use lnd::lnrpc::routerrpc::SendPaymentRequest;

use rand::RngExt;

use sha2::{Sha256, Digest};

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::vec::Vec;

pub const TLV_KEYSEND: u64 = 5482373484;

pub struct LNDClient {
    client: lnd::Lnd,
}

#[async_trait]
impl LNClient for LNDClient {

    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error + Send + Sync>> {
        let info = lnd::Lnd::get_info(&mut self.client).await?;

        Ok(NodeInfo {
            pubkey: info.identity_pubkey,
            alias: info.alias,
            version: info.version,
            nodetype: "LND".to_string(),
        })
    }

    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error + Send + Sync>> {
        let balance = lnd::Lnd::channel_balance(&mut self.client).await?;
        let mut current_balance: i64 = 0;

        if let Some(bal) = balance.local_balance {
            current_balance = bal.sat as i64;
        }

        Ok(current_balance)
    }

    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error + Send + Sync>> {
        let result = match lnd::Lnd::list_invoices(&mut self.client, false, start, limit, false, 0, 0).await {
            Ok(inv) => inv,
            Err(_) => {
                return Err("unable to fetch invoices".into());
            }
        };

        let mut invoices: Vec<Invoice> = Vec::new();

        for item in result.invoices {
            invoices.push(parse_invoice(item));
        }


        Ok(invoices)
    }

    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error + Send + Sync>> {
        let result = match lnd::Lnd::list_payments(&mut self.client, false, start, limit, false, false, 0, 0).await {
            Ok(inv) => inv,
            Err(_) => {
                return Err("unable to fetch payments".into());
            }
        };

        let mut payments: Vec<Payment> = Vec::new();

        for item in result.payments {
            payments.push(parse_payment(&item));
        }

        Ok(payments)
    }

    async fn subscribe_invoices(&mut self, start_index: u64) -> Result<InvoiceStream, Box<dyn Error + Send + Sync>> {
        let mut client = self.client.clone();
        let invoices = lnd::Lnd::subscribe_invoices(&mut client, start_index, 0).await?;

        let stream = invoices.map(|item| match item {
            Ok(invoice) => Ok(parse_invoice(invoice)),
            Err(status) => Err(LNClientError(status.to_string())),
        });

        Ok(Box::pin(stream))
    }

    async fn send_keysend(&mut self, destination: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>> {
        // convert pub key hash to raw bytes
        let raw_pubkey = HEXLOWER.decode(destination.as_bytes())?;
    
        // generate 32 random bytes for pre_image
        let mut pre_image = [0u8; 32];
        {
            let mut rng = rand::rng();
            rng.fill(&mut pre_image);
        }
    
        // and convert to sha256 hash
        let payment_hash = Sha256::digest(&pre_image);

        // add keysend tlv
        let mut dest_custom_records = custom_records.clone();
        dest_custom_records.insert(TLV_KEYSEND, pre_image.to_vec());

        let req = SendPaymentRequest {
            dest: raw_pubkey,
            amt: sats as i64,
            payment_hash: payment_hash.to_vec(),
            dest_custom_records,
            timeout_seconds: 60,
            ..Default::default()
        };

        send_payment(&mut self.client, req).await
    }

    async fn send_bolt11(&mut self, payment_request: String, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error + Send + Sync>> {
        let req = SendPaymentRequest {
            payment_request,
            dest_custom_records: custom_records,
            timeout_seconds: 60,
            ..Default::default()
        };

        send_payment(&mut self.client, req).await
    }
}

pub async fn connect(node_address: String, cert_path: String, macaroon_path: String) -> Result<LNDClient, Box<dyn Error + Send + Sync>> {
    let cert: Vec<u8> = match fs::read(cert_path.clone()) {
        Ok(cert) => cert,
        Err(e) => {
            return Err(Box::new(
                LNClientError(format!("Unable to open certificate file {}: {}", cert_path, e)))
            );
        },
    };

    let macaroon: Vec<u8> = match fs::read(macaroon_path.clone()) {
        Ok(macaroon) => macaroon,
        Err(e) => {
            return Err(Box::new(
                LNClientError(format!("Unable to open macaroon file {}: {}", macaroon_path, e)))
            );
        },
    };

    //Make the connection to LND
    let client = lnd::Lnd::connect_with_macaroon(node_address.clone(), &cert, &macaroon).await?;

    Ok(LNDClient {
        client,
    })
}

fn parse_payment(item: &LndPayment) -> Payment {
    let mut pay = Payment {
        index: item.payment_index,
        time: item.creation_time_ns / 1000000000,
        amount: item.value_sat,

        destination: String::new(), // hop pubkey
        payment_hash: item.payment_hash.clone(),
        payment_preimage: item.payment_preimage.clone(),

        fee: item.fee_sat,

        custom_records: HashMap::new(),
    };

    for htlc in &item.htlcs {
        if htlc.route.is_none() {
            continue; // no route found
        }

        let route = htlc.route.clone().unwrap();
        let last_idx = route.hops.len() - 1;
        let hop = route.hops[last_idx].clone();

        pay.destination = hop.pub_key.clone();

        for (key, value) in &hop.custom_records {
            pay.custom_records.insert(*key, value.clone());
        }
    }

    return pay;
}

fn parse_invoice(item: LndInvoice) -> Invoice {
    let payment_hash = HEXLOWER.encode(&item.r_hash);
    let preimage = HEXLOWER.encode(&item.r_preimage);

    let mut inv = Invoice {
        index: item.add_index,
        time: item.settle_date,
        amount: item.amt_paid_sat,
        memo: item.memo,
        payment_hash,
        preimage,
        custom_records: HashMap::new(),
        settled: item.state == InvoiceState::Settled as i32,
    };

    for htlc in item.htlcs {
        for (key, value) in &htlc.custom_records {
            inv.custom_records.insert(*key, value.clone());
        }
    }

    inv
}

async fn send_payment(client: &mut lnd::Lnd, payment_request: SendPaymentRequest) -> Result<Payment, Box<dyn Error + Send + Sync>> {
    println!("Sending payment to: {:#?}", payment_request);

    // send payment using send_payment_v2 and get payment stream
    let mut payment_stream = client.send_payment_v2(payment_request).await?;

    // wait for payment to succeed or fail
    while let Some(payment_update) = payment_stream.message().await? {
        if payment_update.status == PaymentStatus::Succeeded as i32 {
            return Ok(parse_payment(&payment_update));
        }
        else if payment_update.status == PaymentStatus::Failed as i32 {
            return Err(Box::new(LNClientError("Payment failed".into())));
        }
    }

    Err(Box::new(LNClientError("Payment timed out".into())))
}