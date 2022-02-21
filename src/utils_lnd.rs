use crate::Config;
use crate::config::ResultExt;
use std::error::Error;
use std::env;
use std::fs;
use lnd;

#[macro_use]
extern crate configure_me;

//Configure_me
include_config!();

const HELIPAD_CONFIG_FILE: &str = "./helipad.conf";
const HELIPAD_DATABASE_RECEIVED: &str = "database-received.db";
const HELIPAD_DATABASE_SENT: &str = "database-sent.db";
const HELIPAD_STANDARD_PORT: &str = "2112";
const LND_STANDARD_GRPC_URL: &str = "https://127.0.0.1:10009";
const LND_STANDARD_MACAROON_LOCATION: &str = "/lnd/data/chain/bitcoin/mainnet/admin.macaroon";
const LND_STANDARD_TLSCERT_LOCATION: &str = "/lnd/tls.cert";


pub async fn get_macaroon(helipad_config: config::Config) -> Vec<u8> {
    //Get the macaroon file.  Look in the local directory first as an override.
    //If the file is not found in the currect working directory, look for it at the
    //normal LND directory locations
    println!("\nDiscovering macaroon file path...");
    let macaroon_path;
    let env_macaroon_path = std::env::var("LND_ADMINMACAROON");
    //First try from the environment
    if env_macaroon_path.is_ok() {
        macaroon_path = env_macaroon_path.unwrap();
        println!(" - Trying environment var(LND_ADMINMACAROON): [{}]", macaroon_path);
    } else if helipad_config.macaroon.is_some() {
        macaroon_path = helipad_config.macaroon.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, macaroon_path);
    } else {
        macaroon_path = "admin.macaroon".to_string();
        println!(" - Trying current directory: [{}]", macaroon_path);
    }
    let macaroon: Vec<u8>;
    match fs::read(macaroon_path.clone()) {
        Ok(macaroon_content) => {
            println!(" - Success.");
            macaroon = macaroon_content;
        }
        Err(_) => {
            println!(" - Error reading macaroon from: [{}]", macaroon_path);
            println!(" - Last fallback attempt: [{}]", LND_STANDARD_MACAROON_LOCATION);
            match fs::read(LND_STANDARD_MACAROON_LOCATION) {
                Ok(macaroon_content) => {
                    macaroon = macaroon_content;
                }
                Err(_) => {
                    eprintln!("Cannot find a valid admin.macaroon file");
                    std::process::exit(1);
                }
            }
        }
    }
    return macaroon;
}

pub async fn get_cert(helipad_config: config::Config) -> Vec<u8> {
    println!("\nDiscovering certificate file path...");
    let cert_path;
    let env_cert_path = std::env::var("LND_TLSCERT");
    if env_cert_path.is_ok() {
        cert_path = env_cert_path.unwrap();
        println!(" - Trying environment var(LND_TLSCERT): [{}]", cert_path);
    } else if helipad_config.cert.is_some() {
        cert_path = helipad_config.cert.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, cert_path);
    } else {
        cert_path = "tls.cert".to_string();
        println!(" - Trying current directory: [{}]", cert_path);
    }
    let cert: Vec<u8>;
    match fs::read(cert_path.clone()) {
        Ok(cert_content) => {
            println!(" - Success.");
            cert = cert_content;
        }
        Err(_) => {
            println!(" - Error reading certificate from: [{}]", cert_path);
            println!(" - Last fallback attempt: [{}]", LND_STANDARD_TLSCERT_LOCATION);
            match fs::read(LND_STANDARD_TLSCERT_LOCATION) {
                Ok(cert_content) => {
                    cert = cert_content;
                }
                Err(_) => {
                    eprintln!("Cannot find a valid tls.cert file");
                    std::process::exit(2);
                }
            }
        }
    }
    return cert;
}

pub async fn get_node_address(helipad_config: config::Config) -> String {
    //Get the url connection string of the lnd node
    println!("\nDiscovering LND node address...");
    let node_address;
    let env_lnd_url = std::env::var("LND_URL");
    if env_lnd_url.is_ok() {
        node_address = "https://".to_owned() + env_lnd_url.unwrap().as_str();
        println!(" - Trying environment var(LND_URL): [{}]", node_address);
    } else if helipad_config.lnd_url.is_some() {
        node_address = helipad_config.lnd_url.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, node_address);
    } else {
        node_address = String::from(LND_STANDARD_GRPC_URL);
        println!(" - Trying localhost default: [{}].", node_address);
    }
    return node_address;
}
