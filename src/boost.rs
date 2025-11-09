use serde::Deserialize;
use std::error::Error;
use std::collections::HashMap;
use crate::podcastindex;
use crate::metadata;
use dbif::map_action_to_code;
use lnd::lnrpc::lnrpc::{Payment, Invoice, invoice::InvoiceState};
use crate::deserializers::{d_action, d_blank, d_zero, de_optional_string_or_number};

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
    pub action: Option<String>,

    #[serde(default = "d_blank")]
    pub app_name: Option<String>,

    #[serde(default = "d_blank")]
    pub message: Option<String>,

    #[serde(default = "d_blank")]
    pub sender_name: Option<String>,

    #[serde(default = "d_blank")]
    pub podcast: Option<String>,

    #[serde(default = "d_blank")]
    pub episode: Option<String>,

    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    pub value_msat: Option<u64>,

    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    pub value_msat_total: Option<u64>,

    #[serde(default = "d_blank")]
    pub remote_feed_guid: Option<String>,

    #[serde(default = "d_blank")]
    pub remote_item_guid: Option<String>,

    #[serde(default = "d_blank")]
    pub tlv: Option<String>,
}

impl RawBoost {
    pub fn from_json(json: &str) -> Result<Self, Box<dyn Error>> {
        let mut rawboost = serde_json::from_str::<RawBoost>(json)?;
        rawboost.tlv = Some(json.to_string());
        Ok(rawboost)
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let json = std::str::from_utf8(bytes)?;
        Self::from_json(json)
    }
}

pub async fn parse_boost_from_invoice(invoice: Invoice, remote_cache: &mut podcastindex::GuidCache, fetch_metadata: bool) -> Option<dbif::BoostRecord> {
    if invoice.state != InvoiceState::Settled as i32 {
        return None; // invoice hasn't been fulfilled yet
    }

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

    for htlc in &invoice.htlcs {
        if htlc.custom_records.contains_key(&TLV_PODCASTING20) {
            // Parse boost and custodial wallet TLVs
            parse_custom_records(&mut boost, &htlc.custom_records, remote_cache).await;
            return Some(boost);
        }
    }

    if invoice.payment_request.is_empty() {
        return None; // unrelated keysend/amp payment
    }

    // Fetch any RSS payment or Podcast Guru payment metadata from the invoice memo
    if fetch_metadata && fetch_boost_metadata(&mut boost, &invoice.memo, remote_cache).await {
        return Some(boost);
    }

    // Else use what we have for a "Lightning Invoice" boost
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
        parse_custom_records(&mut boost, &hop.custom_records, remote_cache).await;
        return Some(boost);
    }

    None
}


async fn parse_custom_records(boost: &mut dbif::BoostRecord, custom_records: &HashMap<u64, Vec<u8>>, remote_cache: &mut podcastindex::GuidCache) {
    // Parse boost and custodial wallet TLVs
    for (key, val) in custom_records {
        if *key == TLV_PODCASTING20 {
            // Parse boost TLV
            let rawboost = match RawBoost::from_json_bytes(val) {
                Ok(rawboost) => rawboost,
                Err(e) => {
                    eprintln!("** Error parsing boost TLV: {}", e);
                    continue;
                }
            };
            map_rawboost_to_boost(rawboost, boost, remote_cache).await;
        }
        else if *key == TLV_WALLET_KEY || *key == TLV_WALLET_ID || *key == TLV_HIVE_ACCOUNT || *key == TLV_FOUNTAIN_KEY {
            // Parse custodial wallet info
            if let Ok(custom_value) = std::str::from_utf8(val) {
                boost.custom_key = Some(*key);
                boost.custom_value = Some(custom_value.to_string());
            }
        }
    }
}

pub async fn fetch_boost_metadata(boost: &mut dbif::BoostRecord, comment: &str, remote_cache: &mut podcastindex::GuidCache) -> bool {
    let metadata = match metadata::fetch_payment_metadata(comment).await {
        Ok(Some(metadata)) => metadata,
        Ok(None) => {
            eprintln!("** No payment metadata found for boost: {}", boost.index);
            return false;
        },
        Err(e) => {
            eprintln!("** Error fetching payment metadata: {}", e);
            return false;
        }
    };

    map_rawboost_to_boost(metadata, boost, remote_cache).await;
    true
}

pub async fn map_rawboost_to_boost(rawboost: RawBoost, boost: &mut dbif::BoostRecord, remote_cache: &mut podcastindex::GuidCache) {
    // Determine an action type for later filtering ability
    boost.action = 0;
    if let Some(action) = rawboost.action {
        boost.action = map_action_to_code(&action);
    }

    //Was a sender name given in the tlv?
    boost.sender = rawboost.sender_name.unwrap_or_default();

    //Was there a message in this tlv?
    boost.message = rawboost.message.unwrap_or_default();

    //Was an app name given?
    boost.app = rawboost.app_name.unwrap_or_default();

    //Was a podcast name given?
    boost.podcast = rawboost.podcast.unwrap_or_default();

    //Episode name?
    boost.episode = rawboost.episode.unwrap_or_default();

    //Look for an original sat value in the tlv
    boost.value_msat_total = rawboost.value_msat_total.unwrap_or_default() as i64;

    // Copy the tlv from the rawboost to the boost record
    boost.tlv = rawboost.tlv.unwrap_or_default();

    // Fetch podcast/episode name if remote feed/item GUID present
    populate_remote_guids(
        boost,
        rawboost.remote_feed_guid,
        rawboost.remote_item_guid,
        remote_cache,
    ).await;
}

async fn populate_remote_guids(
    boost: &mut dbif::BoostRecord,
    remote_feed_guid: Option<String>,
    remote_item_guid: Option<String>,
    remote_cache: &mut podcastindex::GuidCache,
) {
    let feed_guid = remote_feed_guid.unwrap_or_default();
    let item_guid = remote_item_guid.unwrap_or_default();

    if !feed_guid.is_empty() {
        if !item_guid.is_empty() {
            if let Ok(guid) = remote_cache.get(feed_guid.clone(), item_guid).await {
                boost.remote_podcast = guid.podcast;
                boost.remote_episode = guid.episode;
            }
        } else {
            // no free api to look up just the feed guid
            boost.remote_podcast = Some(feed_guid);
            boost.remote_episode = None;
        }
    }
}
