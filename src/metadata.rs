use std::error::Error;
use reqwest;
use urlencoding::decode as url_decode;
use regex::Regex;
use serde::Deserialize;
use crate::boost::RawBoost;
use crate::deserializers::{d_blank, d_zero, de_optional_string_or_number};

#[derive(Debug)]
pub struct MetadataError(String);

impl std::fmt::Display for MetadataError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error: {}", self.0)
    }
}

impl std::error::Error for MetadataError {}

#[derive(Deserialize, Debug)]
pub struct RssPayment {
    #[serde(default = "d_blank")]
    action: Option<String>, // String("STREAM"),

    #[serde(default = "d_blank")]
    app_name: Option<String>, // String("Fountain"),

    #[serde(default = "d_blank")]
    feed_title: Option<String>, // String("Podcasting 2.0"),

    #[serde(default = "d_blank")]
    item_title: Option<String>, // String("Episode 241: RSS NutJobs"),

    #[serde(default = "d_blank")]
    message: Option<String>, // Null,

    #[serde(default = "d_blank")]
    remote_feed_guid: Option<String>, // Null,

    #[serde(default = "d_blank")]
    remote_item_guid: Option<String>, // Null,

    #[serde(default = "d_blank")]
    sender_name: Option<String>, // String("rpodcast@fountain.fm"),

    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat_total: Option<u64>, // Number(20000),
}

impl RssPayment {
    pub fn from_json(json: &str) -> Result<Self, Box<dyn Error>> {
        let rss_payment = serde_json::from_str::<RssPayment>(json)?;
        Ok(rss_payment)
    }
}

#[derive(Deserialize, Debug)]
pub struct PodcastGuruPayment {
    metadata_payload: Option<String>,
}

pub async fn fetch_payment_metadata(comment: &str) -> Result<Option<RawBoost>, Box<dyn Error>> {
    let rss_payment_regex = Regex::new(r"rss::payment::\w+ (https:\/\/fountain\.fm\/[^\s]+)")?;

    if let Some(captures) = rss_payment_regex.captures(comment) {
        if let Some(url) = captures.get(1) {
            println!("Found RSS Payment URL: {}", url.as_str());
            return fetch_rss_payment(url.as_str()).await;
        }
    }

    let podcast_guru_regex = Regex::new(r"V4V: (https:\/\/boost\.podcastguru\.io\/[^\s]+)")?;

    if let Some(captures) = podcast_guru_regex.captures(comment) {
        if let Some(url) = captures.get(1) {
            println!("Found Podcast Guru URL: {}", url.as_str());
            return fetch_podcast_guru_payment(url.as_str()).await;
        }
    }

    Ok(None)
}

pub async fn fetch_rss_payment(url: &str) -> Result<Option<RawBoost>, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .head(url)
        .send()
        .await?;

    let x_rss_payment = response
        .headers()
        .get("x-rss-payment")
        .ok_or(Box::new(MetadataError("RSS Payment header not found".into())))?;

    let x_rss_payment_value = x_rss_payment.to_str()?;
    let decoded = url_decode(x_rss_payment_value).expect("UTF-8");
    println!("X-RSS-Payment: {}", decoded);

    let rss_payment = RssPayment::from_json(&decoded)?;
    println!("RSS Payment: {:#?}", rss_payment);

    Ok(Some(RawBoost {
        action: rss_payment.action,
        app_name: rss_payment.app_name,
        podcast: rss_payment.feed_title,
        episode: rss_payment.item_title,
        message: rss_payment.message,
        remote_feed_guid: rss_payment.remote_feed_guid,
        remote_item_guid: rss_payment.remote_item_guid,
        sender_name: rss_payment.sender_name,
        value_msat: None,
        value_msat_total: rss_payment.value_msat_total,
        tlv: Some(decoded.to_string()),
    }))
}

/**
 * Fetch Podcast Guru payment metadata via HTTP GET request
 */
async fn fetch_podcast_guru_payment(url: &str) -> Result<Option<RawBoost>, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .get(url)
        .send()
        .await?;

    let body = response.text().await?;
    println!("Podcast Guru Payment Body: {}", body);
    let payment: PodcastGuruPayment = serde_json::from_str(&body)?;
    println!("Podcast Guru Payment: {:#?}", payment);

    match payment.metadata_payload {
        Some(metadata_json) => match RawBoost::from_json(&metadata_json) {
            Ok(boost) => Ok(Some(boost)),
            Err(e) => Err(Box::new(MetadataError(format!("Error parsing Podcast Guru payment: {}", e)))),
        },
        None => Ok(None),
    }
}