use crate::podcastindex;
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

    if invoice.payment_request.is_empty() {
        return None; // unrelated keysend/amp payment
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