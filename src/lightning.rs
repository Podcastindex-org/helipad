use crate::boost::{TLV_PODCASTING20, TLV_KEYSEND};
use crate::lnaddress::{LnAddress, KeysendAddress, LnurlpAddress};
use data_encoding::HEXLOWER;
use lnd::lnrpc::lnrpc::{Payment, payment::PaymentStatus};
use lnd::lnrpc::routerrpc::{SendPaymentRequest};
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::error::Error;
use rand::RngCore;

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

pub async fn connect_lnd_or_exit(node_address: &str, cert_path: &str, macaroon_path: &str) -> lnd::Lnd {
    match connect_to_lnd(node_address, cert_path, macaroon_path).await {
        Some(lndconn) => lndconn,
        None => std::process::exit(1),
    }
}

async fn create_boost_request(addr: LnAddress, sats: u64, tlv: Value) -> Result<SendPaymentRequest, Box<dyn Error>> {
    // figure out the destination pubkey/lnaddress
    match addr {
        LnAddress::Keysend(keysend) => {
            create_keysend_request(keysend, sats, tlv)
        }
        LnAddress::Lnurlp(lnurlp) => {
            create_bolt11_request(lnurlp, sats, tlv).await
        }
        LnAddress::None => Err(Box::new(BoostError("Destination not found".into())) as Box<dyn Error>),
    }
}

fn create_keysend_request(addr: KeysendAddress, sats: u64, tlv: Value) -> Result<SendPaymentRequest, Box<dyn Error>> {
    // thanks to BrianOfLondon and Mostro for keysend details:
    // https://peakd.com/@brianoflondon/lightning-keysend-is-strange-and-how-to-send-keysend-payment-in-lightning-with-the-lnd-rest-api-via-python
    // https://github.com/MostroP2P/mostro/blob/52a4f86c3942c26bd42dc55f1e53db5da9f7542b/src/lightning/mod.rs#L18

    // convert pub key hash to raw bytes
    let raw_pubkey = HEXLOWER.decode(addr.pubkey.as_bytes())?;

    // generate 32 random bytes for pre_image
    let mut pre_image = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut pre_image);

    // and convert to sha256 hash
    let payment_hash = Sha256::digest(&pre_image);

    // TLV custom records
    // https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md
    let mut dest_custom_records = HashMap::new();
    let tlv_json = serde_json::to_string_pretty(&tlv).unwrap();

    dest_custom_records.insert(TLV_PODCASTING20, tlv_json.as_bytes().to_vec());
    dest_custom_records.insert(TLV_KEYSEND, pre_image.to_vec());

    if let (Some(custom_key), Some(custom_value)) = (addr.custom_key, addr.custom_value) {
        dest_custom_records.insert(custom_key, custom_value.as_bytes().to_vec());
    }

    Ok(SendPaymentRequest {
        dest: raw_pubkey,
        amt: sats as i64,
        payment_hash: payment_hash.to_vec(),
        dest_custom_records,
        timeout_seconds: 60,
        ..Default::default()
    })
}

async fn create_bolt11_request(addr: LnurlpAddress, sats: u64, tlv: Value) -> Result<SendPaymentRequest, Box<dyn Error>> {
    let sender_name = tlv["sender_name"].as_str().unwrap_or_default().to_string();
    let comment = tlv["message"].as_str().unwrap_or_default().to_string();

    let payment_request = match addr.request_invoice(sats, comment, sender_name).await? {
        Some(payment_request) => payment_request,
        None => return Err(Box::new(BoostError("Payment request not found".into()))),
    };

    // TLV custom records
    // https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md
    let mut dest_custom_records = HashMap::new();
    let tlv_json = serde_json::to_string_pretty(&tlv).unwrap();

    dest_custom_records.insert(TLV_PODCASTING20, tlv_json.as_bytes().to_vec());

    Ok(SendPaymentRequest {
        payment_request,
        dest_custom_records,
        timeout_seconds: 60,
        ..Default::default()
    })
}

pub async fn send_boost(lightning: lnd::Lnd, address: String, custom_key: Option<u64>, custom_value: Option<String>, sats: u64, tlv: Value) -> Result<Payment, Box<dyn Error>> {
    let dest = LnAddress::resolve(address, custom_key, custom_value).await?;
    let req = create_boost_request(dest, sats, tlv).await?;
    send_payment(lightning, req).await
}

pub async fn send_payment(mut lightning: lnd::Lnd, payment_request: SendPaymentRequest) -> Result<Payment, Box<dyn Error>> {
    println!("Sending payment to: {:#?}", payment_request);

    // send payment using send_payment_v2 and get payment stream
    let mut payment_stream = lightning.send_payment_v2(payment_request).await?;

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
