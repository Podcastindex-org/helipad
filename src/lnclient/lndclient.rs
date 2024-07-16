use async_trait::async_trait;
use crate::lightning;

use crate::lnclient::{LNClient, NodeInfo, Invoice, Payment};
use crate::lightning::parse_podcast_tlv;

use data_encoding::HEXLOWER;

use lnd::lnrpc::lnrpc::{SendRequest, Payment as LndPayment};

use rand::RngCore;

use sha2::{Sha256, Digest};

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::vec::Vec;


pub struct LNDClient {
    client: lnd::Lnd,
    // remote_cache: GuidCache,
}

impl LNDClient {

    pub async fn connect(node_address: String, cert_path: String, macaroon_path: String) -> Result<LNDClient, Box<dyn Error>> {

        let cert: Vec<u8> = fs::read(cert_path.clone())?;
        let macaroon: Vec<u8> = fs::read(macaroon_path.clone())?;

        //Make the connection to LND
        let client = lnd::Lnd::connect_with_macaroon(node_address.clone(), &cert, &macaroon).await?;

        // let remote_cache = podcastindex::GuidCache::new(1000);

        Ok(LNDClient {
            client,
            // remote_cache
        })
    }

    fn parse_payment(&mut self, item: &LndPayment) -> Payment {
        let mut pay = Payment {
            index: item.payment_index,
            time: item.creation_time_ns / 1000000000,
            amount: item.value_sat,

            destination: String::new(), // hop pubkey
            payment_hash: item.payment_hash.clone(),
            payment_preimage: item.payment_preimage.clone(),

            fee: item.fee_sat,

            custom_records: HashMap::new(),
            boostagram: None,
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

        if let Some(val) = pay.custom_records.get(&lightning::TLV_PODCASTING20) {
            pay.boostagram = Some(parse_podcast_tlv(&val));
        }

        return pay;
    }
}

#[async_trait]
impl LNClient for LNDClient {

    async fn get_info(&mut self) -> Result<NodeInfo, Box<dyn Error>> {
        let info = lnd::Lnd::get_info(&mut self.client).await?;

        Ok(NodeInfo {
            pubkey: info.identity_pubkey,
            alias: info.alias,
            version: info.version,
            nodetype: "LND".to_string(),
        })
    }

    async fn channel_balance(&mut self) -> Result<i64, Box<dyn Error>> {
        let balance = lnd::Lnd::channel_balance(&mut self.client).await?;
        let mut current_balance: i64 = 0;

        if let Some(bal) = balance.local_balance {
            current_balance = bal.sat as i64;
        }

        Ok(current_balance)
    }

    async fn list_invoices(&mut self, start: u64, limit: u64) -> Result<Vec<Invoice>, Box<dyn Error>> {
        let result = match lnd::Lnd::list_invoices(&mut self.client, false, start, limit, false).await {
            Ok(inv) => inv,
            Err(_) => {
                return Err("unable to fetch invoices".into());
            }
        };

        let mut invoices: Vec<Invoice> = Vec::new();

        for item in result.invoices {
            let payment_hash = HEXLOWER.encode(&item.r_hash);
            let preimage = HEXLOWER.encode(&item.r_preimage);

            let mut inv = Invoice {
                index: item.add_index,
                time: item.settle_date,
                amount: item.amt_paid_sat,
                payment_hash: payment_hash,
                preimage: preimage,
                custom_records: HashMap::new(),
                boostagram: None,
            };

            for htlc in item.htlcs {
                for (key, value) in &htlc.custom_records {
                    inv.custom_records.insert(*key, value.clone());
                }
            }

            if let Some(val) = inv.custom_records.get(&lightning::TLV_PODCASTING20) {
                inv.boostagram = Some(parse_podcast_tlv(&val));
            }

            invoices.push(inv);
        }


        Ok(invoices)
    }

    async fn list_payments(&mut self, start: u64, limit: u64) -> Result<Vec<Payment>, Box<dyn Error>> {
        let result = match lnd::Lnd::list_payments(&mut self.client, false, start, limit, false).await {
            Ok(inv) => inv,
            Err(_) => {
                return Err("unable to fetch payments".into());
            }
        };

        let mut payments: Vec<Payment> = Vec::new();

        for item in result.payments {
            payments.push(self.parse_payment(&item));
        }

        Ok(payments)
    }

    async fn keysend(&mut self, destination: String, sats: u64, custom_records: HashMap<u64, Vec<u8>>) -> Result<Payment, Box<dyn Error>> { // -> Result<Payment, Box<dyn Error>> {
       // convert pub key hash to raw bytes
        let raw_pubkey = HEXLOWER.decode(destination.as_bytes()).unwrap();

        // generate 32 random bytes for pre_image
        let mut pre_image = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut pre_image);

        // and convert to sha256 hash
        let mut hasher = Sha256::new();
        hasher.update(pre_image);
        let payment_hash = hasher.finalize();

        // add pre_image to custom_record for keysend
        let mut custom_records = custom_records.clone();
        custom_records.insert(lightning::TLV_KEYSEND, pre_image.to_vec());

        // assemble the lnd payment
        let req = SendRequest {
            dest: raw_pubkey.clone(),
            amt: sats as i64,
            payment_hash: payment_hash.to_vec(),
            dest_custom_records: custom_records,
            ..Default::default()
        };

        // send payment and get payment hash
        let response = lnd::Lnd::send_payment_sync(&mut self.client, req).await?;
        let sent_payment_hash = HEXLOWER.encode(&response.payment_hash);

        if response.payment_error != "" {
            return Err(response.payment_error.into());
        }

        // get detailed payment info from list_payments
        let payment_list = lnd::Lnd::list_payments(&mut self.client, false, 0, 500, true).await?;

        for payment in payment_list.payments {
            if sent_payment_hash == payment.payment_hash {
                return Ok(self.parse_payment(&payment));
            }
        }

        Err("Failed to find payment sent".into())
    }
}
