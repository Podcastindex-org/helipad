use std::error::Error;
use std::fs;

use dbif;
use lnd;
extern crate hex;

const HELIPAD_CONFIG_FILE: &str = "./helipad.conf";
const HELIPAD_DATABASE_DIR: &str = "database.db";
const HELIPAD_STANDARD_PORT: &str = "2112";
const LND_STANDARD_GRPC_URL: &str = "https://127.0.0.1:10009";
const LND_STANDARD_MACAROON_LOCATION: &str = "/lnd/data/chain/bitcoin/mainnet/admin.macaroon";
const LND_STANDARD_TLSCERT_LOCATION: &str = "/lnd/tls.cert";


pub async fn get_macaroon(macaroon_path_config_file: String) -> Vec<u8> {
    //Get the macaroon file. Look in the local directory first as an override.
    //If the file is not found in the currect working directory, look for it at the
    //normal LND directory locations
    println!("\nDiscovering macaroon file path...");
    let macaroon_path;
    let env_macaroon_path = std::env::var("LND_ADMINMACAROON");
    //First try from the environment
    if env_macaroon_path.is_ok() {
        macaroon_path = env_macaroon_path.unwrap();
        println!(" - Trying environment var(LND_ADMINMACAROON): [{}]", macaroon_path);
    } else if macaroon_path_config_file.len() > 0 {
        macaroon_path = macaroon_path_config_file;
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

pub async fn get_cert(cert_path_config_file: String) -> Vec<u8> {
    println!("\nDiscovering certificate file path...");
    let cert_path;
    let env_cert_path = std::env::var("LND_TLSCERT");
    if env_cert_path.is_ok() {
        cert_path = env_cert_path.unwrap();
        println!(" - Trying environment var(LND_TLSCERT): [{}]", cert_path);
    } else if cert_path_config_file.len() > 0 {
        cert_path = cert_path_config_file;
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

pub async fn get_node_address(lnd_url_config_file: String) -> String {
    //Get the url connection string of the lnd node
    println!("\nDiscovering LND node address...");
    let node_address;
    let env_lnd_url = std::env::var("LND_URL");
    if env_lnd_url.is_ok() {
        node_address = "https://".to_owned() + env_lnd_url.unwrap().as_str();
        println!(" - Trying environment var(LND_URL): [{}]", node_address);
    } else if lnd_url_config_file.len() > 0 {
        node_address = lnd_url_config_file;
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, node_address);
    } else {
        node_address = String::from(LND_STANDARD_GRPC_URL);
        println!(" - Trying localhost default: [{}].", node_address);
    }
    return node_address;
}

pub async fn get_node_info(mut connection: lnd::Lnd) -> String {
    let node_info: String;
    node_info = "".to_string();
    match lnd::Lnd::get_info(&mut connection).await {
        Ok(node_info) => {
            println!("LND node info: {:#?}", node_info);
        }
        Err(e) => {
            eprintln!("Error getting LND node info: {:#?}", e);
        }
    }
    return node_info;
}

pub async fn get_balance(mut connection: lnd::Lnd) -> i64 {
    let mut current_balance: i64 = 0;

    match lnd::Lnd::channel_balance(&mut connection).await {
        Ok(balance) => {
            if let Some(bal) = balance.local_balance {
                println!("LND node local balance: {:#?}", bal.sat);
                current_balance = bal.sat as i64;
            }
        }
        Err(e) => {
            eprintln!("Error getting LND wallet balance: {:#?}", e);
        }
    }
    return current_balance;
}

pub async fn get_connection(cert_path_config_file: String, macaroon_path_config_file: String, lnd_url_config_file: String) -> lnd::Lnd {
    let lightning;
    let cert_path: Vec<u8>;
    let macaroon: Vec<u8>;
    let node_address;

    cert_path = get_cert(cert_path_config_file).await;
    macaroon = get_macaroon(macaroon_path_config_file).await;
    node_address = get_node_address(lnd_url_config_file).await;

    match lnd::Lnd::connect_with_macaroon(node_address.clone(), &cert_path, &macaroon).await {
        Ok(lndconn) => {
            println!(" - Success.");
            lightning = lndconn;
        }
        Err(e) => {
            //println!("Could not connect to: [{}] using tls: [{}] and macaroon: [{}]", node_address, cert_path, macaroon);
            eprintln!("{:#?}", e);
            std::process::exit(1);
        }
    }
    return lightning;
}

pub async fn send_boostagram(cert_path_config_file: String, macaroon_path_config_file: String, lnd_url_config_file: String, db_filepath: String, feed_id: String, feed_url: String, recipient: String, podcast: String, episode: String, episode_time_seconds: i64, sender: String, message: String, node_address_destination: String, amount_sat: i64) -> bool {
    let mut sent: bool = false;
    let mut connection: lnd::Lnd = get_connection(cert_path_config_file, macaroon_path_config_file, lnd_url_config_file).await;
    let mut lnd_request: lnd::lnrpc::lnrpc::SendRequest;

    let amount_msat = amount_sat * 1000;

    //Initialize a boostagram record
    let mut boost = dbif::BoostRecord {
        index: 0,
        action: 2,
        app: "helipad".to_string(),
        podcast: podcast.to_string(),
        episode: episode.to_string(),
        time: episode_time_seconds,
        sender: sender.to_string(),
        value_msat: amount_msat,
        value_msat_total: amount_msat,
        message: message.to_string(),
        tlv: "unknown".to_string(),
    };

    // TODO: Create lnd request

    // Create json string
    let action = "boost";
    let app_name = "helipad";

    let tlv_json = format!("{{ \"action\": \"{}\", \"app_name\": \"{}\", \"sender_name\": \"{}\", \"feedID\": {}, \"url\": \"{}\", \"podcast\": \"{}\", \"episode\": \"{}\", \"name\": \"{}\", \"ts\": {}, \"value_msat\": {}, \"value_msat_total\": {}, \"message\": \"{}\" }}\n", action, app_name, sender, feed_id, feed_url, podcast, episode, recipient, episode_time_seconds, amount_msat, amount_msat, message);
    

    // Encode tlv json to hex
    boost.tlv = hex::encode(tlv_json);

    // TODO: Send boostagram
//    match lnd::Lnd::send_payment_sync(&mut connection, lnd_request).await {
//        Ok(result) => {
//            sent = true;
//
//            //TODO: Store in the database
//            match dbif::add_invoice_to_db(&db_filepath, boost) {
//                 Ok(_) => println!("New invoice added."),
//                 Err(e) => eprintln!("Error adding invoice: {:#?}", e)
//            }
//
//        }
//        Err(e) => {
//            eprintln!("Error sending boostagram: {:#?}", e);
//        }
//    }
    return sent;
}
