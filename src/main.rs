//Modules ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
use axum::{
    http::Method,
    middleware,
    routing::{get, post, delete, patch},
    Router,
};

use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use chrono::Utc;
use drop_root::set_user_group;
use rand::{distributions::Alphanumeric, Rng}; // 0.8

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT, HeaderMap, HeaderValue};
use reqwest::redirect::Policy;

use std::env;
use std::fs;
use std::path::Path;

#[macro_use]
extern crate configure_me;


//Globals ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
mod handler;
mod lightning;
mod podcastindex;
mod lnclient;

const HELIPAD_CONFIG_FILE: &str = "./helipad.conf";
const HELIPAD_DATABASE_DIR: &str = "database.db";
const HELIPAD_SOUND_DIR: &str = "./sounds";
const HELIPAD_STANDARD_PORT: &str = "2112";

const LND_STANDARD_GRPC_URL: &str = "https://127.0.0.1:10009";
const LND_STANDARD_MACAROON_LOCATION: &str = "/lnd/data/chain/bitcoin/mainnet/admin.macaroon";
const LND_STANDARD_TLSCERT_LOCATION: &str = "/lnd/tls.cert";

const CLN_URL: &str = "https://127.0.0.1:2105";
const CLN_CERT_PATH: &str = "/cln/regtest/client.pem";
const CLN_KEY_PATH: &str = "/cln/regtest/client-key.pem";
const CLN_CACERT_PATH: &str = "/cln/regtest/ca.pem";

const REMOTE_GUID_CACHE_SIZE: usize = 20;

const WEBROOT_PATH_IMAGE: &str = "webroot/image";
const WEBROOT_PATH_STYLE: &str = "webroot/style";
const WEBROOT_PATH_SCRIPT: &str = "webroot/script";

//Structs ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct AppState {
    pub helipad_config: HelipadConfig,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct HelipadConfig {
    pub database_file_path: String,
    pub sound_path: String,
    pub listen_port: String,
    pub password: String,
    pub secret: String,
    pub node_type: String,
    pub lnd_url: String,
    pub lnd_macaroon_path: String,
    pub lnd_cert_path: String,
    pub cln_url: String,
    pub cln_cert_path: String,
    pub cln_key_path: String,
    pub cln_cacert_path: String,
}

//Configure_me
include_config!();

//Main -------------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
#[tokio::main]
async fn main() {
    //Get what version we are
    let version = env!("CARGO_PKG_VERSION");
    println!("Version: {}", version);
    println!("--------------------");

    //Configuration
    let mut helipad_config = HelipadConfig {
        database_file_path: "".to_string(),
        sound_path: "".to_string(),
        listen_port: "".to_string(),
        password: "".to_string(),
        secret: "".to_string(),
        node_type: "".to_string(),
        lnd_url: "".to_string(),
        lnd_macaroon_path: "".to_string(),
        lnd_cert_path: "".to_string(),
        cln_url: "".to_string(),
        cln_cert_path: "".to_string(),
        cln_key_path: "".to_string(),
        cln_cacert_path: "".to_string(),
    };

    //Bring in the configuration info
    let (server_config, _remaining_args) = Config::including_optional_config_files(&[HELIPAD_CONFIG_FILE]).unwrap_or_exit();

    //Debugging
    println!("Config file(database_dir): {:#?}", server_config.database_dir);
    println!("Config file(sound_dir): {:#?}", server_config.sound_dir);
    println!("Config file(listen_port): {:#?}", server_config.listen_port);
    println!("Config file(macaroon): {:#?}", server_config.macaroon);
    println!("Config file(cert): {:#?}", server_config.cert);

    //LISTEN PORT -----
    println!("\nDiscovering listen port...");
    let mut listen_port = String::from(HELIPAD_STANDARD_PORT);
    let args: Vec<String> = env::args().collect();
    let env_listen_port = std::env::var("HELIPAD_LISTEN_PORT");
    //First try from the environment
    if env_listen_port.is_ok() {
        listen_port = env_listen_port.unwrap();
        println!(" - Using environment var(HELIPAD_LISTEN_PORT): [{}]", listen_port);
    } else if server_config.listen_port.is_some() {
        //If that fails, try from the config file
        listen_port = server_config.listen_port.unwrap().to_string();
        println!(" - Using config file({}): [{}]", HELIPAD_CONFIG_FILE, listen_port);
    } else if let Some(arg_port) = args.get(1) {
        //If that fails, try from the command line
        listen_port = arg_port.to_owned();
        println!(" - Using arg from command line: [{}]", listen_port);
    } else {
        //If everything fails, then just use the default port
        println!(" - Nothing else found. Using default: [{}]...", listen_port);
    }
    helipad_config.listen_port = listen_port.clone();

    //DATABASE FILE -----
    //First try to get the database file location from the environment
    println!("\nDiscovering database location...");
    let env_database_file_path = std::env::var("HELIPAD_DATABASE_DIR");
    if env_database_file_path.is_ok() {
        helipad_config.database_file_path = env_database_file_path.unwrap();
        println!(" - Using environment var(HELIPAD_DATABASE_DIR): [{}]", helipad_config.database_file_path);
    } else {
        //If that fails, try to get it from the config file
        if server_config.database_dir.is_some() {
            helipad_config.database_file_path = server_config.database_dir.clone().unwrap().to_string();
            println!(" - Using config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.database_file_path);
        } else {
            //If that fails just fall back to the local directory
            helipad_config.database_file_path = HELIPAD_DATABASE_DIR.to_string();
            println!(" - Nothing else found. Using default: [{}]", helipad_config.database_file_path);
        }
    }
    //Create the database file
    match dbif::create_database(&helipad_config.database_file_path) {
        Ok(_) => {
            println!("Database file is ready...");
        }
        Err(e) => {
            eprintln!("Database error: {:#?}", e);
            std::process::exit(3);
        }
    }

    //SOUND DIR
    //Get the directory to store boost sounds in
    println!("\nDiscovering sound directory...");
    if let Ok(sound_dir) = std::env::var("HELIPAD_SOUND_DIR") {
        helipad_config.sound_path = sound_dir.clone();
        println!(" - Using environment var(HELIPAD_SOUND_DIR): [{}]", helipad_config.sound_path);
    }
    else if let Some(sound_dir) = server_config.sound_dir {
        helipad_config.sound_path = sound_dir.clone();
        println!(" - Using config var(sound_dir): [{}]", helipad_config.sound_path);
    }
    else {
        helipad_config.sound_path = HELIPAD_SOUND_DIR.to_string();
        println!(" - Using default: [{}]", helipad_config.sound_path);
    }

    if !Path::new(&helipad_config.sound_path).is_dir() {
        if let Err(e) = fs::create_dir_all(&helipad_config.sound_path) {
            eprintln!("Unable to create sound directory: {}", e);
        }
    }

    //PASSWORD -----
    //Get the configured password for Helipad
    let env_password = std::env::var("HELIPAD_PASSWORD");
    if env_password.is_ok() {
        helipad_config.password = env_password.unwrap();
        println!("Found password in environment var(HELIPAD_PASSWORD)");
    } else if server_config.password.is_some() {
        helipad_config.password = server_config.password.unwrap();
        println!("Found password in config file({})", HELIPAD_CONFIG_FILE);
    }

    //Generate secret for JWT if password set
    if !helipad_config.password.is_empty() {
        helipad_config.secret = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect();
    }

    //Get the url connection string of the CLN node if provided
    println!("\nDiscovering node type...");

    let env_node_type = std::env::var("NODE_TYPE");

    if env_node_type.is_ok() {
        helipad_config.node_type = env_node_type.unwrap();
        println!(" - Trying environment var(NODE_TYPE): [{}]", helipad_config.node_type);
    } else if server_config.node_type.is_some() {
        helipad_config.node_type = server_config.node_type.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.node_type);
    }

    if helipad_config.node_type.to_uppercase() == "CLN" {
        println!("\nDiscovering CLN GRPC address...");
        let env_cln_url = std::env::var("CLN_URL");

        if env_cln_url.is_ok() {
            helipad_config.cln_url = "https://".to_owned() + env_cln_url.unwrap().as_str();
            println!(" - Trying environment var(CLN_URL): [{}]", helipad_config.cln_url);
        } else if server_config.cln_url.is_some() {
            helipad_config.cln_url = server_config.cln_url.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.cln_url);
        } else {
            helipad_config.cln_cert_path = String::from(CLN_URL);
            println!(" - Trying default: [{}]", helipad_config.cln_url);
        }

        //Get the rune string of the CLN node
        println!("\nDiscovering CLN GRPC certificate path...");
        let env_cln_cert_path = std::env::var("CLN_CERT_PATH");

        if env_cln_cert_path.is_ok() {
            helipad_config.cln_cert_path = env_cln_cert_path.unwrap();
            println!(" - Trying environment var(CLN_CERT_PATH): [{}]", helipad_config.cln_cert_path);
        } else if server_config.cln_cert_path.is_some() {
            helipad_config.cln_cert_path = server_config.cln_cert_path.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.cln_cert_path);
        } else {
            helipad_config.cln_cert_path = String::from(CLN_CERT_PATH);
            println!(" - Trying default: [{}]", helipad_config.cln_cert_path);
        }

        println!("\nDiscovering CLN GRPC CA key path...");
        let env_cln_key_path = std::env::var("CLN_KEY_PATH");

        if env_cln_key_path.is_ok() {
            helipad_config.cln_key_path = env_cln_key_path.unwrap();
            println!(" - Trying environment var(CLN_KEY_PATH): [{}]", helipad_config.cln_key_path);
        } else if server_config.cln_key_path.is_some() {
            helipad_config.cln_key_path = server_config.cln_key_path.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.cln_key_path);
        } else {
            helipad_config.cln_key_path = String::from(CLN_KEY_PATH);
            println!(" - Trying default: [{}]", helipad_config.cln_key_path);
        }

        println!("\nDiscovering CLN GRPC root certificate path...");
        let env_cln_cacert_path = std::env::var("CLN_CACERT_PATH");

        if env_cln_cacert_path.is_ok() {
            helipad_config.cln_cacert_path = env_cln_cacert_path.unwrap();
            println!(" - Trying environment var(CLN_CACERT_PATH): [{}]", helipad_config.cln_cacert_path);
        } else if server_config.cln_cacert_path.is_some() {
            helipad_config.cln_cacert_path = server_config.cln_cacert_path.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.cln_cacert_path);
        } else {
            helipad_config.cln_cacert_path = String::from(CLN_CACERT_PATH);
            println!(" - Trying default: [{}]", helipad_config.cln_cacert_path);
        }
    }
    else {
        //Get the url connection string of the lnd node
        println!("\nDiscovering LND node address...");
        let env_lnd_url = std::env::var("LND_URL");

        if env_lnd_url.is_ok() {
            helipad_config.lnd_url = "https://".to_owned() + env_lnd_url.unwrap().as_str();
            println!(" - Trying environment var(LND_URL): [{}]", helipad_config.lnd_url);
        } else if server_config.lnd_url.is_some() {
            helipad_config.lnd_url = server_config.lnd_url.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.lnd_url);
        } else {
            helipad_config.lnd_url = String::from(LND_STANDARD_GRPC_URL);
            println!(" - Trying localhost default: [{}].", helipad_config.lnd_url);
        }

        //Get the macaroon and cert files.  Look in the local directory first as an override.
        //If the files are not found in the currect working directory, look for them at their
        //normal LND directory locations
        println!("\nDiscovering macaroon file path...");
        let env_macaroon_path = std::env::var("LND_ADMINMACAROON");

        //First try from the environment
        if env_macaroon_path.is_ok() {
            helipad_config.lnd_macaroon_path = env_macaroon_path.unwrap();
            println!(" - Trying environment var(LND_ADMINMACAROON): [{}]", helipad_config.lnd_macaroon_path);
        } else if server_config.macaroon.is_some() {
            helipad_config.lnd_macaroon_path = server_config.macaroon.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.lnd_macaroon_path);
        } else if Path::new("admin.macaroon").is_file() {
            helipad_config.lnd_macaroon_path = "admin.macaroon".to_string();
            println!(" - Trying current directory: [{}]", helipad_config.lnd_macaroon_path);
        } else {
            helipad_config.lnd_macaroon_path = String::from(LND_STANDARD_MACAROON_LOCATION);
            println!(" - Trying LND default: [{}]", helipad_config.lnd_macaroon_path);
        }

        println!("\nDiscovering certificate file path...");
        let env_cert_path = std::env::var("LND_TLSCERT");

        if env_cert_path.is_ok() {
            helipad_config.lnd_cert_path = env_cert_path.unwrap();
            println!(" - Trying environment var(LND_TLSCERT): [{}]", helipad_config.lnd_cert_path);
        } else if server_config.cert.is_some() {
            helipad_config.lnd_cert_path = server_config.cert.unwrap();
            println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.lnd_cert_path);
        } else if Path::new("tls.cert").is_file() {
            helipad_config.lnd_cert_path = "tls.cert".to_string();
            println!(" - Trying current directory: [{}]", helipad_config.lnd_cert_path);
        } else {
            helipad_config.lnd_cert_path = String::from(LND_STANDARD_TLSCERT_LOCATION);
            println!(" - Trying LND default: [{}]", helipad_config.lnd_cert_path);
        }
    }

    //Start the LND polling thread.  This thread will poll LND every few seconds to
    //get the latest invoices and store them in the database.
    tokio::spawn(lnd_poller(helipad_config.clone()));

    //App State
    let state = AppState {
        helipad_config: helipad_config.clone(),
        version: version.to_string(),
    };

    // Api routes

    //Router
    let app = Router::new()
        // authed routes (if password set)
        .nest("/", Router::new()
            .route("/", get(handler::home))
            .route("/streams", get(handler::streams))
            .route("/sent", get(handler::sent))
            .route("/settings", get(handler::settings))
            .route("/numerology.json", get(handler::numerology_json))

            .route("/settings/general", get(handler::general_settings_load))
            .route("/settings/general", post(handler::general_settings_save))

            .route("/settings/numerology", get(handler::numerology_settings_list))
            .route("/settings/numerology/reset", get(handler::numerology_settings_reset))
            .route("/settings/numerology/reset", post(handler::numerology_settings_do_reset))
            .route("/settings/numerology/:idx", patch(handler::numerology_settings_patch))
            .route("/settings/numerology/:idx", get(handler::numerology_settings_load))
            .route("/settings/numerology/:idx", post(handler::numerology_settings_save))
            .route("/settings/numerology/:idx", delete(handler::numerology_settings_delete))

            .route("/settings/webhooks", get(handler::webhook_settings_list))
            .route("/settings/webhooks/:idx", get(handler::webhook_settings_load))
            .route("/settings/webhooks/:idx", post(handler::webhook_settings_save))
            .route("/settings/webhooks/:idx", delete(handler::webhook_settings_delete))

            .route("/csv", get(handler::csv_export_boosts))

            // public api (cors all origins)
            .nest("/api/v1", Router::new()
                .route("/node_info", get(handler::api_v1_node_info))
                .route("/settings", get(handler::api_v1_settings))
                .route("/boosts", get(handler::api_v1_boosts))
                .route("/balance", get(handler::api_v1_balance))
                .route("/streams", get(handler::api_v1_streams))
                .route("/sent", get(handler::api_v1_sent))
                .route("/index", get(handler::api_v1_index))
                .route("/sent_index", get(handler::api_v1_sent_index))

                // allow all origins to GET from public api
                .route_layer(CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any))
            )

            // protected api
            .route("/api/v1/reply", post(handler::api_v1_reply))
            .route("/api/v1/mark_replied", post(handler::api_v1_mark_replied))

            // require auth for above routes
            .route_layer(middleware::from_fn_with_state(state.clone(), handler::auth_middleware))
        )

        // login page
        .route("/login", get(handler::login).post(handler::handle_login))

        // static assets
        .nest_service("/image", ServeDir::new(WEBROOT_PATH_IMAGE))
        .nest_service("/script", ServeDir::new(WEBROOT_PATH_SCRIPT))
        .nest_service("/style", ServeDir::new(WEBROOT_PATH_STYLE))
        .nest_service("/sound", ServeDir::new(helipad_config.sound_path))

        .nest_service("/pew.mp3", ServeFile::new("webroot/extra/pew.mp3"))
        .nest_service("/favicon.ico", ServeFile::new("webroot/extra/favicon.ico"))
        .nest_service("/apps.json", ServeFile::new("webroot/extra/apps.json"))

        .with_state(state);


    let binding = format!("0.0.0.0:{}", &listen_port);
    let listener = tokio::net::TcpListener::bind(&binding).await.unwrap();

    println!("\nHelipad is listening on http://{}", binding);
    axum::serve(listener, app).await.unwrap();

    //If a "run as" user is set in the "HELIPAD_RUN_AS" environment variable, then switch to that user
    //and drop root privileges after we've bound to the low range socket
    match env::var("HELIPAD_RUNAS_USER") {
        Ok(runas_user) => {
            match set_user_group(runas_user.as_str(), "nogroup") {
                Ok(_) => {
                    println!("RunAs: {}", runas_user.as_str());
                }
                Err(e) => {
                    eprintln!("RunAs Error: {} - Check that your HELIPAD_RUNAS_USER env var is set correctly.", e);
                }
            }
        }
        Err(_) => {
            eprintln!("ALERT: Use the HELIPAD_RUNAS_USER env var to avoid running as root.");
        }
    }
}

//The LND poller runs in a thread and pulls new invoices
async fn lnd_poller(helipad_config: HelipadConfig) {
    let db_filepath = helipad_config.database_file_path.clone();

    //Make the connection to LND
    println!("\nConnecting to node address...");

    let mut lightning = match lnclient::connect(&helipad_config).await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Unable to connect to node: {}", e);
            return;
        }
    };

    println!(" - Success.");

    //Get lnd node info
    match lightning.get_info().await {
        Ok(node_info) => {
            println!("Node info: {:#?}", node_info);

            let record = dbif::NodeInfoRecord {
                lnd_alias: node_info.alias,
                node_pubkey: node_info.pubkey,
                node_version: node_info.version,
            };

            if dbif::add_node_info_to_db(&db_filepath, record).is_err() {
                println!("Error updating node info in database.");
            }
        }
        Err(e) => {
            eprintln!("Error getting node info: {:#?}", e);
        }
    }

    //Instantiate a cache to use when resolving remote podcasts/episode guids
    let mut remote_cache = podcastindex::GuidCache::new(REMOTE_GUID_CACHE_SIZE);

    //The main loop
    let mut current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();
    let mut current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();

    loop {
        let mut updated = false;

        //Get lnd node channel balance
        match lightning.channel_balance().await {
            Ok(current_balance) => {
                if dbif::add_wallet_balance_to_db(&db_filepath, current_balance).is_err() {
                    println!("Error adding wallet balance to the database.");
                }
            },
            Err(e) => {
                eprintln!("Error getting wallet balance: {:#?}", e);
            }
        };

        //Get a list of invoices
        let invoices = match lightning.list_invoices(current_index.clone(), 500).await {
            Ok(invoices) => invoices,
            Err(e) => {
                eprintln!("lightning::list_invoices failed: {}", e);
                vec![]
            }
        };

        for invoice in invoices {
            if let Some(db_boost) = lightning::parse_boost_from_invoice(&invoice, &mut remote_cache).await {
                //Give some output
                println!("Boost: {:#?}", &db_boost);

                //Store in the database
                match dbif::add_invoice_to_db(&db_filepath, &db_boost) {
                    Ok(_) => println!("New invoice added."),
                    Err(e) => eprintln!("Error adding invoice: {:#?}", e)
                }

                //Send out webhooks (if any)
                send_webhooks(&db_filepath, &db_boost).await;
            }

            current_index = invoice.index;
            updated = true;
        }

        //Make sure we are tracking our position properly
        println!("Current index: {}", current_index);

        let payments = match lightning.list_payments(current_payment, 500).await {
            Ok(payments) => payments,
            Err(e) => {
                eprintln!("lightning::list_payments failed: {}", e);
                vec![]
            }
        };

        for payment in payments {
            let parsed = lightning::parse_boost_from_payment(&payment, &mut remote_cache).await;

            if let Some(boost) = parsed {
                //Give some output
                println!("Sent Boost: {:#?}", boost);

                //Store in the database
                match dbif::add_payment_to_db(&db_filepath, &boost) {
                    Ok(_) => println!("New payment added."),
                    Err(e) => eprintln!("Error adding payment: {:#?}", e)
                }

                //Send out webhooks (if any)
                send_webhooks(&db_filepath, &boost).await;
            }

            current_payment = payment.index;
            updated = true;
        }

        //Make sure we are tracking our position properly
        println!("Current payment: {}", current_payment);

        //Sleep only if nothing was updated
        if !updated {
            tokio::time::sleep(tokio::time::Duration::from_millis(9000)).await;
        }
    }
}

async fn send_webhooks(db_filepath: &String, boost: &dbif::BoostRecord) {
    let webhooks = match dbif::get_webhooks_from_db(&db_filepath, Some(true)) {
        Ok(wh) => wh,
        Err(e) => {
            eprintln!("Error loading webhooks from db: {:#?}", e);
            return;
        }
    };

    for webhook in webhooks {
        if boost.payment_info.is_some() && !webhook.on_sent {
            continue; // sent
        }

        if boost.action == 1 && !webhook.on_stream {
            continue; // stream
        }

        if (boost.action == 2 || boost.action == 4) && !webhook.on_boost {
            continue; // boost or auto
        }

        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, match HeaderValue::from_str("application/json") {
            Ok(hdr) => hdr,
            Err(e) => {
                eprintln!("Unable to create content type header: {}", e);
                continue;
            }
        });

        let user_agent = format!("Helipad/{}", env!("CARGO_PKG_VERSION"));
        headers.insert(USER_AGENT, match HeaderValue::from_str(user_agent.as_str()) {
            Ok(hdr) => hdr,
            Err(e) => {
                eprintln!("Unable to create user agent header: {}", e);
                continue;
            }
        });

        if webhook.token != "" {
            let token = format!("Bearer {}", webhook.token);
            headers.insert(AUTHORIZATION, match HeaderValue::from_str(&token) {
                Ok(hdr) => hdr,
                Err(e) => {
                    eprintln!("Unable to create authorization header: {}", e);
                    continue;
                }
            });
        }

        let client = match reqwest::Client::builder().redirect(Policy::limited(5)).build() {
            Ok(cli) => cli,
            Err(e) => {
                eprintln!("Unable to build reqwest client: {}", e);
                continue;
            }
        };

        let json = match serde_json::to_string_pretty(&boost) {
            Ok(js) => js,
            Err(e) => {
                eprintln!("Unable to encode boost as JSON: {}", e);
                continue;
            }
        };

        let result = client.post(&webhook.url).body(json).headers(headers).send().await;
        let timestamp = Utc::now().timestamp();
        let mut successful = false;

        if let Ok(res) = result {
            let status = res.status();
            let response = res.text().await;

            if status == 200 && response.is_ok() {
                println!("Webhook sent to {}: {}", webhook.url, response.unwrap());
                successful = true;
            }
            else if status != 200 {
                eprintln!("Webhook returned {}: {}", status, response.unwrap_or_default());
            }
            else if let Err(e) = response {
                eprintln!("Webhook Error: {}", e);
            }
        }
        else if let Err(e) = result {
            eprintln!("Unable to send webhook: {}", e);
        }

        if let Err(e) = dbif::set_webhook_last_request(&db_filepath, webhook.index, successful, timestamp) {
            eprintln!("Error setting webhook last request status: {}", e);
        }
    }
}