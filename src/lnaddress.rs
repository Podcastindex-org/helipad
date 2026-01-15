use serde::{Serialize, Deserialize};
use std::error::Error;
use reqwest;
use urlencoding::encode as url_encode;
use serde_json::json;

pub enum LnAddress {
    Keysend(KeysendAddress),
    Lnurlp(LnurlpAddress),
    None, // no LnAddress
}

impl LnAddress {
    pub async fn resolve(address: String, custom_key: Option<u64>, custom_value: Option<String>) -> Result<LnAddress, Box<dyn Error>> {
        if !address.contains('@') {
            return Ok(LnAddress::Keysend(KeysendAddress::new(address, custom_key, custom_value)));
        }

        if let Some(data) = KeysendAddress::resolve(&address).await? {
            return Ok(LnAddress::Keysend(data));
        }

        if let Some(data) = LnurlpAddress::resolve(&address).await? {
            return Ok(LnAddress::Lnurlp(data));
        }

        Ok(LnAddress::None)
    }
}

pub struct KeysendAddress {
    pub pubkey: String,
    pub custom_key: Option<u64>,
    pub custom_value: Option<String>,
}

impl KeysendAddress {
    pub fn new(pubkey: String, custom_key: Option<u64>, custom_value: Option<String>) -> Self {
        Self { pubkey, custom_key, custom_value }
    }

    pub async fn resolve(address: &str) -> Result<Option<KeysendAddress>, Box<dyn Error>> {
        let (username, hostname) = parse_lnaddress(address)?;
        let url = format!("https://{}/.well-known/keysend/{}", hostname, username);

        let text = match fetch_endpoint(&url).await? {
            Some(t) => t,
            None => return Ok(None), // 404 - doesn't exist
        };

        let data: KeysendAddressResponse = serde_json::from_str(&text)?;

        println!("Keysend {}: pubkey={}", address, data.pubkey);
        for item in &data.custom_data {
            println!("  {}={}", item.custom_key, item.custom_value);
        }

        // assume the first custom key/value pair is the wallet
        let (custom_key, custom_value) = match data.custom_data.first() {
            Some(item) => {
                (Some(item.custom_key.parse::<u64>()?), Some(item.custom_value.clone()))
            }
            None => {
                (None, None)
            }
        };

        Ok(Some(KeysendAddress {
            pubkey: data.pubkey,
            custom_key,
            custom_value,
        }))
    }
}

pub struct LnurlpAddress {
    pub comment_allowed: u32,
    pub callback: String,
    pub min_sendable: u64,
    pub max_sendable: u64,
    pub payer_data: Option<LnurlpPayerData>,
}

impl LnurlpAddress {
    pub async fn resolve(address: &str) -> Result<Option<LnurlpAddress>, Box<dyn Error>> {
        let (username, hostname) = parse_lnaddress(address)?;
        let url = format!("https://{}/.well-known/lnurlp/{}", hostname, username);

        let text = match fetch_endpoint(&url).await? {
            Some(t) => t,
            None => return Ok(None), // 404 - doesn't exist
        };

        let data: LnurlpResponse = serde_json::from_str(&text)?;

        println!("Lnurlp {}: {:#?}", address, data);

        Ok(Some(LnurlpAddress {
            comment_allowed: data.comment_allowed,
            callback: data.callback,
            min_sendable: data.min_sendable,
            max_sendable: data.max_sendable,
            payer_data: data.payer_data,
        }))
    }

    pub async fn request_invoice(self, sats: u64, comment: String, sender_name: String) -> Result<Option<String>, Box<dyn Error>> {
        // Limit comment to comment_allowed characters
        let comment_len = self.comment_allowed.min(comment.len() as u32) as usize;
        let limited_comment = &comment[..comment_len];

        let mut url = format!("{}?amount={}", self.callback, sats * 1000);
        if !limited_comment.is_empty() {
            url.push_str(&format!("&comment={}", url_encode(limited_comment)));
        }

        if let Some(payer_data) = &self.payer_data {
            if let Some(_) = &payer_data.name {
                let pd = json!({"name": sender_name});
                let pd = serde_json::to_string(&pd).unwrap();
                url.push_str(&format!("&payerdata={}", url_encode(&pd)));
            }
        }

        if sats > self.max_sendable / 1000 {
            return Err(format!("Amount is greater than max sendable: {}", self.max_sendable / 1000).into());
        }

        if sats < self.min_sendable / 1000 {
            return Err(format!("Amount is less than min sendable: {}", self.min_sendable / 1000).into());
        }

        println!("Lnurlp Callback URL: {}", url);

        let text = reqwest::get(&url).await?.text().await?;
        let data: LnurlpCallbackResponse = serde_json::from_str(&text)?;

        println!("Lnurlp Callback: {:#?}", data);

        Ok(data.pr)
    }

}


// Response types
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct KeysendAddressResponse {
    status: String,
    tag: String,
    pubkey: String,
    custom_data: Vec<KeysendAddressCustomData>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct KeysendAddressCustomData {
    custom_key: String,
    custom_value: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LnurlpResponse {
    status: String,
    tag: String,
    #[serde(default)]
    comment_allowed: u32,
    callback: String,
    metadata: Option<String>,
    min_sendable: u64,
    max_sendable: u64,
    payer_data: Option<LnurlpPayerData>,
    #[serde(default)]
    nostr_pubkey: String,
    #[serde(default)]
    allows_nostr: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LnurlpPayerData {
    name: Option<LnurlpPayerItem>,
    email: Option<LnurlpPayerItem>,
    pubkey: Option<LnurlpPayerItem>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LnurlpPayerItem {
    #[serde(default)]
    mandatory: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct LnurlpCallbackResponse {
    pr: Option<String>,
}

// Helper functions
fn parse_lnaddress(address: &str) -> Result<(&str, &str), Box<dyn Error>> {
    let (username, hostname) = address.split_once('@')
        .ok_or("Invalid lightning address format")?;

    if username.is_empty() || hostname.is_empty() {
        return Err("Invalid lightning address format".into());
    }

    Ok((username, hostname))
}

async fn fetch_endpoint(url: &str) -> Result<Option<String>, Box<dyn Error>> {
    let response = reqwest::get(url).await?;

    // Return None for 404 (endpoint doesn't exist)
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    // Return error for other non-success statuses
    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()).into());
    }

    Ok(Some(response.text().await?))
}