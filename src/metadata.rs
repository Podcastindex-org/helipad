use std::error::Error;
use reqwest;
use reqwest::redirect::Policy;
use url::Url;
use urlencoding::decode as url_decode;
use regex::Regex;
use serde::Deserialize;
use crate::boost::RawBoost;
use crate::deserializers::{d_blank, d_zero, de_optional_string_or_number};

// Helipad will automatically fetch metadata from these domains
const METADATA_PAYMENT_DOMAINS: &[&str] = &[
    "fountain.fm",
    "castamatic.com",
    "tardbox.com",
    "podfans.fm",
    "truefans.fm",
    "v4vmusic.com",
];

const PODCAST_GURU_DOMAIN: &str = "boost.podcastguru.io";
const MAX_REDIRECTS: usize = 5;

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

#[derive(Debug, Clone, PartialEq)]
pub enum PaymentUrl {
    RssPayment(Url),
    PodcastGuru(Url),
}

fn allowed_domains(additional_domains: &str) -> Vec<String> {
    let mut domains: Vec<String> = METADATA_PAYMENT_DOMAINS.iter().map(|d| d.to_string()).collect();

    domains.extend(
        additional_domains
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
    );

    domains
}

fn is_allowed_url(url: &Url, allowed_domains: &[String]) -> bool {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
        return false;
    }

    match url.host_str() {
        Some(host) => allowed_domains.iter().any(|d| d.eq_ignore_ascii_case(host)),
        None => false,
    }
}

fn find_allowed_url(comment: &str, regex: &Regex, allowed_domains: &[String]) -> Option<Url> {
    regex.captures_iter(comment)
        .filter_map(|captures| captures.get(1))
        .filter_map(|m| Url::parse(m.as_str()).ok())
        .find(|url| is_allowed_url(url, allowed_domains))
}

pub fn find_payment_url(comment: &str, metadata_whitelist: &str) -> Option<PaymentUrl> {
    let allowed_domains = allowed_domains(metadata_whitelist);
    let rss_payment_regex = Regex::new(r"rss::payment::\w+ (https://\S+)").ok()?;

    if let Some(url) = find_allowed_url(comment, &rss_payment_regex, &allowed_domains) {
        return Some(PaymentUrl::RssPayment(url));
    }

    let podcast_guru_regex = Regex::new(r"V4V: (https://\S+)").ok()?;
    let podcast_guru_domains = [PODCAST_GURU_DOMAIN.to_string()];

    find_allowed_url(comment, &podcast_guru_regex, &podcast_guru_domains)
        .map(PaymentUrl::PodcastGuru)
}

pub async fn fetch_payment_metadata(payment_url: &PaymentUrl, metadata_whitelist: &str) -> Result<Option<RawBoost>, Box<dyn Error>> {
    let allowed_domains = allowed_domains(metadata_whitelist);

    match payment_url {
        PaymentUrl::RssPayment(url) => {
            println!("Found RSS Payment URL: {}", url);
            fetch_rss_payment(url.as_str(), &allowed_domains).await
        },
        PaymentUrl::PodcastGuru(url) => {
            println!("Found Podcast Guru URL: {}", url);
            fetch_podcast_guru_payment(url.as_str()).await
        },
    }
}

fn redirect_policy(allowed_domains: &[String]) -> Policy {
    let allowed_domains = allowed_domains.to_vec();

    Policy::custom(move |attempt| {
        if attempt.previous().len() > MAX_REDIRECTS || !is_allowed_url(attempt.url(), &allowed_domains) {
            attempt.stop()
        } else {
            attempt.follow()
        }
    })
}

async fn fetch_rss_payment_url(url: &str, allowed_domains: &[String]) -> Result<String, Box<dyn Error>> {
    let app_version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("Helipad/{}", app_version))
        .redirect(redirect_policy(allowed_domains))
        .build()?;

    let response = client
        .head(url)
        .send()
        .await?;

    let x_rss_payment_value = response
        .headers()
        .get("x-rss-payment")
        .ok_or_else(|| MetadataError("RSS Payment header not found".into()))?;

    Ok(x_rss_payment_value.to_str()?.to_string())
}

async fn fetch_retry_rss_payment_url(url: &str, allowed_domains: &[String]) -> Result<String, Box<dyn Error>> {
    if let Ok(value) = fetch_rss_payment_url(url, allowed_domains).await {
        return Ok(value);
    }

    println!("RSS Payment header not found, retrying in 10 seconds");
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;

    fetch_rss_payment_url(url, allowed_domains).await
}

pub async fn fetch_rss_payment(url: &str, allowed_domains: &[String]) -> Result<Option<RawBoost>, Box<dyn Error>> {
    let x_rss_payment_value = fetch_retry_rss_payment_url(url, allowed_domains).await?;

    let decoded = url_decode(&x_rss_payment_value).expect("UTF-8");
    println!("X-RSS-Payment: {}", decoded);

    let rss_payment = RssPayment::from_json(&decoded)?;
    println!("RSS Payment: {:#?}", rss_payment);

    Ok(Some(RawBoost {
        action: rss_payment.action.map(|action| action.to_lowercase()),
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
    let app_version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(format!("Helipad/{}", app_version))
        .redirect(redirect_policy(&[PODCAST_GURU_DOMAIN.to_string()]))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rss_url(memo: &str, extra: &str) -> Option<String> {
        match find_payment_url(memo, extra) {
            Some(PaymentUrl::RssPayment(url)) => Some(url.to_string()),
            _ => None,
        }
    }

    #[test]
    fn skips_disallowed_url_before_allowed_one() {
        let memo = "rss::payment::boost https://10.0.0.1/admin rss::payment::boost https://fountain.fm/p/abc";
        assert_eq!(rss_url(memo, ""), Some("https://fountain.fm/p/abc".to_string()));
    }

    #[test]
    fn rejects_disallowed_hosts() {
        for url in [
            "https://fountain.fm.evil.com/",
            "https://fountain.fm@evil.com/",
            "https://user@fountain.fm/",
            "https://fountainXfm/",
            "https://fountain.fm:8080/",
            "http://fountain.fm/",
            "https://127.0.0.1/",
            "https://evil.fountain.fm/",
        ] {
            let memo = format!("rss::payment::boost {}", url);
            assert_eq!(find_payment_url(&memo, ""), None, "{}", url);
        }
    }

    #[test]
    fn accepts_case_insensitive_host() {
        assert_eq!(
            rss_url("rss::payment::boost https://FOUNTAIN.fm/p/abc", ""),
            Some("https://fountain.fm/p/abc".to_string()),
        );
    }

    #[test]
    fn accepts_whitelisted_extra_domain() {
        let memo = "rss::payment::boost https://example.com/p/abc";
        assert_eq!(rss_url(memo, ""), None);
        assert_eq!(rss_url(memo, " Example.com , other.org"), Some("https://example.com/p/abc".to_string()));
    }

    #[test]
    fn podcast_guru_host_is_fixed() {
        assert!(matches!(
            find_payment_url("V4V: https://boost.podcastguru.io/abc", ""),
            Some(PaymentUrl::PodcastGuru(_))
        ));

        assert_eq!(find_payment_url("V4V: https://evil.com/abc", ""), None);
        assert_eq!(find_payment_url("V4V: https://fountain.fm/abc", ""), None);
    }
}