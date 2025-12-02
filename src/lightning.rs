use crate::podcastindex;
use data_encoding::HEXLOWER;
use lnd::lnrpc::lnrpc::{Payment, Invoice, payment::PaymentStatus, invoice::InvoiceState};
use lnd::lnrpc::routerrpc::{SendPaymentRequest};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::error::Error;
use rand::RngCore;
use serde::{Serialize, Deserialize, Deserializer};

// TLV keys (see https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md)
pub const TLV_PODCASTING20: u64 = 7629169;
pub const TLV_WALLET_KEY: u64 = 696969;
pub const TLV_WALLET_ID: u64 = 112111100;
pub const TLV_HIVE_ACCOUNT: u64 = 818818;
pub const TLV_FOUNTAIN_KEY: u64 = 906608;
pub const TLV_KEYSEND: u64 = 5482373484;


#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct RawBoost {
    #[serde(default = "d_action")]
    action: Option<String>,
    #[serde(default = "d_blank")]
    app_name: Option<String>,
    #[serde(default = "d_blank")]
    message: Option<String>,
    #[serde(default = "d_blank")]
    sender_name: Option<String>,
    #[serde(default = "d_blank")]
    podcast: Option<String>,
    #[serde(default = "d_blank")]
    episode: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat: Option<u64>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat_total: Option<u64>,
    #[serde(default = "d_blank")]
    remote_feed_guid: Option<String>,
    #[serde(default = "d_blank")]
    remote_item_guid: Option<String>,
}

fn d_action() -> Option<String> {
    Some("stream".to_string())
}

fn d_blank() -> Option<String> {
    None
}

fn d_zero() -> Option<u64> {
    None
}

fn de_optional_string_or_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    Ok(
        match Value::deserialize(deserializer)? {
            Value::String(s) => {
                if s.is_empty() {
                    return Ok(None);
                }
                if let Ok(number) = s.parse() {
                    Some(number)
                } else {
                    return Ok(None);
                }
            }
            Value::Number(num) => {
                if num.is_u64() {
                    if let Some(number) = num.as_u64() {
                        Some(number)
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }

            }
            _ => Some(0)
        }
    )
}



#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeysendAddressResponse {
    status: String,
    tag: String,
    pubkey: String,
    custom_data: Vec<KeysendAddressCustomData>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct KeysendAddressCustomData {
    custom_key: String,
    custom_value: String,
}

#[derive(Debug)]
pub struct KeysendAddressError(String);

impl std::fmt::Display for KeysendAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "There is an error: {}", self.0)
    }
}

impl std::error::Error for KeysendAddressError {}

#[derive(Debug)]
pub struct BoostError(String);

impl std::fmt::Display for BoostError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}

impl std::error::Error for BoostError {}

pub async fn connect_to_lnd(node_address: &str, cert_path: &str, macaroon_path: &str) -> Option<lnd::Lnd> {
    let cert: Vec<u8> = match fs::read(cert_path) {
        Ok(cert_content) => cert_content,
        Err(_) => {
            eprintln!("Cannot find a valid tls.cert file");
            return None;
        }
    };

    let macaroon: Vec<u8> = match fs::read(macaroon_path) {
        Ok(macaroon_content) => macaroon_content,
        Err(_) => {
            eprintln!("Cannot find a valid admin.macaroon file");
            return None;
        }
    };

    //Make the connection to LND
    let address = String::from(node_address);
    let lightning = lnd::Lnd::connect_with_macaroon(address.clone(), &cert, &macaroon).await;

    if lightning.is_err() {
        println!("Could not connect to: [{}] using tls: [{}] and macaroon: [{}]", address, cert_path, macaroon_path);
        eprintln!("{:#?}", lightning.err());
        return None;
    }

    lightning.ok()
}

pub async fn resolve_keysend_address(address: &str) -> Result<KeysendAddressResponse, Box<dyn Error>> {
    if !address.contains('@') {
        return Err(Box::new(KeysendAddressError("Invalid keysend address".to_string())));
    }

    if !email_address::EmailAddress::is_valid(address) {
        return Err(Box::new(KeysendAddressError("Invalid keysend address".to_string())));
    }

    let parts: Vec<&str> = address.split('@').collect();

    if parts.len() != 2 {
        return Err(Box::new(KeysendAddressError("Invalid keysend address".to_string())));
    }

    let url = format!("https://{}/.well-known/keysend/{}", parts[1], parts[0]);
    let response = reqwest::get(url.clone()).await?.text().await?;
    let data: KeysendAddressResponse = serde_json::from_str(&response)?;

    print!("Keysend address {}: pub_key={}", address, data.pubkey);

    for item in &data.custom_data {
        print!(" custom_key={}, custom_value={}", item.custom_key, item.custom_value);
    }

    Ok(data)
}

pub async fn send_boost(mut lightning: lnd::Lnd, destination: String, custom_key: Option<u64>, custom_value: Option<String>, sats: u64, tlv: Value) -> Result<Payment, Box<dyn Error>> {
    // thanks to BrianOfLondon and Mostro for keysend details:
    // https://peakd.com/@brianoflondon/lightning-keysend-is-strange-and-how-to-send-keysend-payment-in-lightning-with-the-lnd-rest-api-via-python
    // https://github.com/MostroP2P/mostro/blob/52a4f86c3942c26bd42dc55f1e53db5da9f7542b/src/lightning/mod.rs#L18

    let recipient_pubkey: String;
    let mut recipient_custom_data: HashMap<u64, String> = HashMap::new();

    // convert keysend address into pub_key/custom keyvalue format
    if destination.contains('@') {
        let ln_info = resolve_keysend_address(&destination).await?;

        recipient_pubkey = ln_info.pubkey;

        for item in ln_info.custom_data {
            let ckey_u64 = item.custom_key.parse::<u64>()?;

            recipient_custom_data.insert(
                ckey_u64,
                item.custom_value.clone()
            );
        }
    }
    else {
        recipient_pubkey = destination;

        if let Some(ckey) = custom_key {
            if let Some(cvalue) = custom_value {
                recipient_custom_data.insert(ckey, cvalue);
            }
        }
    }

    // convert pub key hash to raw bytes
    let raw_pubkey = HEXLOWER.decode(recipient_pubkey.as_bytes()).unwrap();

    // generate 32 random bytes for pre_image
    let mut pre_image = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut pre_image);

    // and convert to sha256 hash
    let mut hasher = Sha256::new();
    hasher.update(pre_image);
    let payment_hash = hasher.finalize();

    // TLV custom records
    // https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md
    let mut dest_custom_records = HashMap::new();
    let tlv_json = serde_json::to_string_pretty(&tlv).unwrap();

    dest_custom_records.insert(TLV_KEYSEND, pre_image.to_vec());
    dest_custom_records.insert(TLV_PODCASTING20, tlv_json.as_bytes().to_vec());

    for (key, value) in recipient_custom_data {
        dest_custom_records.insert(key, value.as_bytes().to_vec());
    }

    // assemble the lnd payment
    let req = SendPaymentRequest {
        dest: raw_pubkey.clone(),
        amt: sats as i64,
        payment_hash: payment_hash.to_vec(),
        dest_custom_records,
        timeout_seconds: 60,
        ..Default::default()
    };

    println!("Sending payment to: {:#?}", req);

    // send payment using send_payment_v2 and get payment stream
    let mut payment_stream = lightning.send_payment_v2(req).await?;

    // wait for payment to succeed or fail
    while let Some(payment_update) = payment_stream.message().await? {
        if payment_update.status == PaymentStatus::Succeeded as i32 {
            return Ok(payment_update);
        }
        else if payment_update.status == PaymentStatus::Failed as i32 {
            return Err(Box::new(BoostError("Payment failed".into())));
        }
    }

    Err(Box::new(BoostError("Payment timed out".into())))
}



pub async fn parse_podcast_tlv(boost: &mut dbif::BoostRecord, val: &[u8], remote_cache: &mut podcastindex::GuidCache) {
    let tlv = std::str::from_utf8(val).unwrap();
    println!("TLV: {:#?}", tlv);

    boost.tlv = tlv.to_string();

    let json_result = serde_json::from_str::<RawBoost>(tlv);
    match json_result {
        Ok(rawboost) => {
            //Determine an action type for later filtering ability
            if rawboost.action.is_some() {
                boost.action = dbif::ActionType::from_str(rawboost.action.unwrap().as_str()) as u8;
            }

            //Was a sender name given in the tlv?
            if rawboost.sender_name.is_some() && !rawboost.sender_name.clone().unwrap().is_empty() {
                boost.sender = rawboost.sender_name.unwrap();
            }

            //Was there a message in this tlv?
            if rawboost.message.is_some() {
                boost.message = rawboost.message.unwrap();
            }

            //Was an app name given?
            if rawboost.app_name.is_some() {
                boost.app = rawboost.app_name.unwrap();
            }

            //Was a podcast name given?
            if rawboost.podcast.is_some() {
                boost.podcast = rawboost.podcast.unwrap();
            }

            //Episode name?
            if rawboost.episode.is_some() {
                boost.episode = rawboost.episode.unwrap();
            }

            //Look for an original sat value in the tlv
            if rawboost.value_msat_total.is_some() {
                boost.value_msat_total = rawboost.value_msat_total.unwrap() as i64;
            }

            //Fetch podcast/episode name if remote feed/item guid present
            let remote_feed_guid = rawboost.remote_feed_guid.unwrap_or_default();
            let remote_item_guid = rawboost.remote_item_guid.unwrap_or_default();

            if !remote_feed_guid.is_empty() {
                if !remote_item_guid.is_empty() {
                    let episode_guid = remote_cache.get(remote_feed_guid, remote_item_guid).await;

                    if let Ok(guid) = episode_guid {
                        boost.remote_podcast = guid.podcast;
                        boost.remote_episode = guid.episode;
                    }
                }
                else {
                    // no free api to look up just the feed guid
                    boost.remote_podcast = Some(remote_feed_guid);
                    boost.remote_episode = None;
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

pub async fn parse_boost_from_invoice(invoice: Invoice, remote_cache: &mut podcastindex::GuidCache) -> Option<dbif::BoostRecord> {
    //Initialize a boost record
    let mut boost = dbif::BoostRecord {
        index: invoice.add_index,
        time: invoice.settle_date,
        value_msat: invoice.amt_paid_sat * 1000,
        value_msat_total: invoice.amt_paid_sat * 1000,
        action: 0,
        sender: "".to_string(),
        app: "".to_string(),
        message: "".to_string(),
        podcast: "".to_string(),
        episode: "".to_string(),
        tlv: "".to_string(),
        remote_podcast: None,
        remote_episode: None,
        reply_sent: false,
        custom_key: None,
        custom_value: None,
        payment_info: None,
    };

    for htlc in invoice.htlcs {
        if !htlc.custom_records.contains_key(&TLV_PODCASTING20) {
            continue; // ignore invoices without a podcasting 2.0 tlv
        }

        // Parse boost and custodial wallet TLVs
        for (key, val) in htlc.custom_records {
            if key == TLV_PODCASTING20 {
                // Parse boost TLV
                parse_podcast_tlv(&mut boost, &val, remote_cache).await;
            }
            else if key == TLV_WALLET_KEY || key == TLV_WALLET_ID || key == TLV_HIVE_ACCOUNT || key == TLV_FOUNTAIN_KEY {
                // Parse custodial wallet info
                let custom_value = std::str::from_utf8(&val).unwrap().to_string();
                boost.custom_key = Some(key);
                boost.custom_value = Some(custom_value);
            }
        }

        return Some(boost);
    }

    if invoice.state != InvoiceState::Settled as i32 {
        return None; // invoice hasn't been fulfilled yet
    }

    // Use what we have for a "Lightning Invoice" boost
    if !invoice.memo.is_empty() {
        boost.action = 5;
        boost.app = "Lightning Invoice".to_string();
        boost.sender = "Lightning Invoice".to_string();
        boost.message = invoice.memo;
        return Some(boost);
    }

    None
}

pub async fn parse_boost_from_payment(payment: Payment, remote_cache: &mut podcastindex::GuidCache) -> Option<dbif::BoostRecord> {

    for htlc in payment.htlcs {

        if htlc.route.is_none() {
            continue; // no route found
        }

        let route = htlc.route.unwrap();
        let hopidx = route.hops.len() - 1;
        let hop = route.hops[hopidx].clone();

        if !hop.custom_records.contains_key(&TLV_PODCASTING20) {
            continue; // not a boost payment
        }

        //Initialize a boost record
        let mut boost = dbif::BoostRecord {
            index: payment.payment_index,
            time: payment.creation_time_ns / 1000000000,
            value_msat: payment.value_msat,
            value_msat_total: payment.value_msat,
            action: 0,
            sender: "".to_string(),
            app: "".to_string(),
            message: "".to_string(),
            podcast: "".to_string(),
            episode: "".to_string(),
            tlv: "".to_string(),
            remote_podcast: None,
            remote_episode: None,
            custom_key: None,
            custom_value: None,
            reply_sent: false,
            payment_info: Some(dbif::PaymentRecord {
                payment_hash: payment.payment_hash.clone(),
                pubkey: hop.pub_key.clone(),
                custom_key: 0,
                custom_value: "".into(),
                fee_msat: payment.fee_msat,
                reply_to_idx: None,
            }),
        };

        // Parse boost and custodial wallet TLVs
        for (key, val) in hop.custom_records {
            if key == TLV_PODCASTING20 {
                // Parse boost TLV
                parse_podcast_tlv(&mut boost, &val, remote_cache).await;
            }
            else if key == TLV_WALLET_KEY || key == TLV_WALLET_ID || key == TLV_HIVE_ACCOUNT || key == TLV_FOUNTAIN_KEY {
                // Parse custodial wallet info
                let custom_value = std::str::from_utf8(&val).unwrap().to_string();

                boost.payment_info = Some(dbif::PaymentRecord {
                    payment_hash: payment.payment_hash.clone(),
                    pubkey: hop.pub_key.clone(),
                    custom_key: key,
                    custom_value,
                    fee_msat: payment.fee_msat,
                    reply_to_idx: None,
                });
            }
        }

        return Some(boost);
    }

    None
}