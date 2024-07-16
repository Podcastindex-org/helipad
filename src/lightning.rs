use crate::podcastindex;
use crate::lnclient::{LNClient, Invoice, Payment, Boost};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use serde::{Deserialize, Deserializer};

// TLV keys (see https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md)
pub const TLV_PODCASTING20: u64 = 7629169;
pub const TLV_WALLET_KEY: u64 = 696969;
pub const TLV_WALLET_ID: u64 = 112111100;
pub const TLV_HIVE_ACCOUNT: u64 = 818818;
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

    println!("");

    return Ok(data);
}

pub async fn send_boost(mut lightning: Box<dyn LNClient>, destination: String, custom_key: Option<u64>, custom_value: Option<String>, sats: u64, tlv: Value) -> Result<Payment, Box<dyn Error>> {
    // thanks to BrianOfLondon and Mostro for keysend details:
    // https://peakd.com/@brianoflondon/lightning-keysend-is-strange-and-how-to-send-keysend-payment-in-lightning-with-the-lnd-rest-api-via-python
    // https://github.com/MostroP2P/mostro/blob/52a4f86c3942c26bd42dc55f1e53db5da9f7542b/src/lightning/mod.rs#L18

    let recipient_pubkey: String;
    let mut recipient_custom_data: HashMap<u64, String> = HashMap::new();

    // convert keysend address into pub_key/custom keyvalue format
    if destination.contains("@") {
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
        recipient_pubkey = destination.clone();

        if custom_key.is_some() && custom_value.is_some() {
            recipient_custom_data.insert(custom_key.unwrap(), custom_value.unwrap());
        }
    }

    // TLV custom records
    // https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md
    let mut dest_custom_records = HashMap::new();
    let tlv_json = serde_json::to_string_pretty(&tlv).unwrap();

    // dest_custom_records.insert(TLV_KEYSEND, pre_image.to_vec());
    dest_custom_records.insert(TLV_PODCASTING20, tlv_json.as_bytes().to_vec());

    for (key, value) in recipient_custom_data {
        dest_custom_records.insert(key, value.as_bytes().to_vec());
    }

    return lightning.keysend(recipient_pubkey, sats, dest_custom_records).await;
}

pub fn parse_podcast_tlv(val: &Vec<u8>) -> Boost {
    let tlv = std::str::from_utf8(&val).unwrap();

    let mut boost = Boost {
        action: 0,
        podcast: "".to_string(),
        episode: "".to_string(),
        message: "".to_string(),
        sender: "".to_string(),
        app: "".to_string(),
        tlv: tlv.to_string(),
        value_msat: 0,
        value_msat_total: 0,
        remote_feed_guid: "".to_string(),
        remote_item_guid: "".to_string(),
    };

    match serde_json::from_str::<RawBoost>(tlv) {
        Ok(rawboost) => {
            //If there was a sat value in the tlv, override the invoice
            if rawboost.value_msat.is_some() {
                boost.value_msat = rawboost.value_msat.unwrap() as i64;
            }

            //Determine an action type for later filtering ability
            if rawboost.action.is_some() {
                boost.action = match rawboost.action.unwrap().as_str() {
                    "stream" => 1, //This indicates a per-minute podcast payment
                    "boost"  => 2, //This is a manual boost or boost-a-gram
                    "auto"   => 4, //This is an automated boost
                    _        => 3, //Invalid action or empty string (set to 3 for legacy reasons)
                }
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
            if rawboost.remote_feed_guid.is_some() {
                boost.remote_feed_guid = rawboost.remote_feed_guid.unwrap();
            }

            if rawboost.remote_item_guid.is_some() {
                boost.remote_item_guid = rawboost.remote_item_guid.unwrap();
            }
        },
        Err(e) => {
            eprintln!("{}", e);
        }
    };

    return boost;
}

pub async fn parse_boost_from_invoice(invoice: &Invoice, remote_cache: &mut podcastindex::GuidCache) -> Option<dbif::BoostRecord> {
    if invoice.boostagram.is_none() {
        return None;
    }

    let boost = invoice.boostagram.clone().unwrap();

    let mut db_boost = dbif::BoostRecord {
        index: invoice.index,
        time: invoice.time,
        value_msat: boost.value_msat,
        value_msat_total: boost.value_msat_total,
        action: boost.action,
        sender: boost.sender,
        app: boost.app,
        message: boost.message,
        podcast: boost.podcast,
        episode: boost.episode,
        tlv: boost.tlv,
        remote_podcast: None,
        remote_episode: None,
        reply_sent: false,
        payment_info: None,
    };

    //Fetch podcast/episode name if remote feed/item guid present
    if boost.remote_feed_guid != "" && boost.remote_item_guid != "" {
        match remote_cache.get(boost.remote_feed_guid, boost.remote_item_guid).await {
            Ok(guid) => {
                db_boost.remote_podcast = guid.podcast;
                db_boost.remote_episode = guid.episode;
            }
            Err(_) => {}
        }
    }

    Some(db_boost)
}


pub async fn parse_boost_from_payment(payment: &Payment, remote_cache: &mut podcastindex::GuidCache) -> Option<dbif::BoostRecord> {
    let payment = payment.clone();

    if payment.boostagram.is_none() {
        return None;
    }

    let boost = payment.boostagram.unwrap();

    let mut db_payment_info = dbif::PaymentRecord {
        payment_hash: payment.payment_hash,
        pubkey: payment.destination,
        custom_key: 0,
        custom_value: "".into(),
        fee_msat: payment.fee * 1000,
        reply_to_idx: None,
    };

    //Get custom key/value for keysend wallet
    for (idx, val) in payment.custom_records {
        if idx == TLV_WALLET_KEY || idx == TLV_WALLET_ID || idx == TLV_HIVE_ACCOUNT {
            let custom_value = std::str::from_utf8(&val).unwrap().to_string();

            db_payment_info.custom_key = idx;
            db_payment_info.custom_value = custom_value;
        }
    }

    //Initialize a boost record
    let mut db_boost = dbif::BoostRecord {
        index: payment.index,
        time: payment.time,
        value_msat: boost.value_msat,
        value_msat_total: boost.value_msat_total,
        action: boost.action,
        sender: boost.sender,
        app: boost.app,
        message: boost.message,
        podcast: boost.podcast,
        episode: boost.episode,
        tlv: boost.tlv,
        remote_podcast: None,
        remote_episode: None,
        reply_sent: false,
        payment_info: Some(db_payment_info),
    };

    //Fetch podcast/episode name if remote feed/item guid present
    if boost.remote_feed_guid != "" && boost.remote_item_guid != "" {
        match remote_cache.get(boost.remote_feed_guid, boost.remote_item_guid).await {
            Ok(guid) => {
                db_boost.remote_podcast = guid.podcast;
                db_boost.remote_episode = guid.episode;
            }
            Err(_) => {}
        }
    }

    Some(db_boost)
}
