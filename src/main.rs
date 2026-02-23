//Modules ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
use axum::{
    http::{Method, header, HeaderValue as AxumHeaderValue},
    middleware,
    routing::{get, post, delete, patch, any},
    Router,
    extract::{State, ws::WebSocket, ws::WebSocketUpgrade},
    response::Response,
};

use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use drop_root::set_user_group;

use serde::Serialize;
use std::env;
use std::fs;
use std::path::Path;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[macro_use]
extern crate configure_me;


//Globals ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
mod handler;
mod lightning;
mod podcastindex;
mod lnaddress;
mod metadata;
mod boost;
mod deserializers;
mod triggers;
mod poller;

const HELIPAD_CONFIG_FILE: &str = "./helipad.conf";
const HELIPAD_DATABASE_DIR: &str = "database.db";
const HELIPAD_SOUND_DIR: &str = "./sounds";
const HELIPAD_STANDARD_PORT: &str = "2112";

const LND_STANDARD_GRPC_URL: &str = "https://127.0.0.1:10009";
const LND_STANDARD_MACAROON_LOCATION: &str = "/lnd/data/chain/bitcoin/mainnet/admin.macaroon";
const LND_STANDARD_TLSCERT_LOCATION: &str = "/lnd/tls.cert";

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
    pub ws_tx: Arc<broadcast::Sender<WebSocketEvent>>,
    pub settings: Arc<RwLock<dbif::SettingsRecord>>,
}

#[derive(Clone, Debug)]
pub struct HelipadConfig {
    pub database_file_path: String,
    pub sound_path: String,
    pub listen_port: String,
    pub macaroon_path: String,
    pub cert_path: String,
    pub node_address: String,
    pub password: String,
    pub secret: String,
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
        macaroon_path: "".to_string(),
        cert_path: "".to_string(),
        node_address: "".to_string(),
        password: "".to_string(),
        secret: "".to_string(),
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
        arg_port.clone_into(&mut listen_port);
        println!(" - Using arg from command line: [{}]", listen_port);
    } else {
        //If everything fails, then just use the default port
        println!(" - Nothing else found. Using default: [{}]...", listen_port);
    }
    helipad_config.listen_port.clone_from(&listen_port);

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
        helipad_config.sound_path.clone_from(&sound_dir);
        println!(" - Using environment var(HELIPAD_SOUND_DIR): [{}]", helipad_config.sound_path);
    }
    else if let Some(sound_dir) = server_config.sound_dir {
        helipad_config.sound_path.clone_from(&sound_dir);
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

    //Get or generate secret for JWT if password set
    if !helipad_config.password.is_empty() {
        helipad_config.secret = match dbif::get_or_create_jwt_secret(&helipad_config.database_file_path) {
            Ok(secret) => secret,
            Err(e) => {
                eprintln!("Warning: Failed to get JWT secret from database: {}", e);
                return;
            }
        };
    }

    //Get the macaroon and cert files.  Look in the local directory first as an override.
    //If the files are not found in the currect working directory, look for them at their
    //normal LND directory locations
    println!("\nDiscovering macaroon file path...");
    let env_macaroon_path = std::env::var("LND_ADMINMACAROON");
    //First try from the environment
    if env_macaroon_path.is_ok() {
        helipad_config.macaroon_path = env_macaroon_path.unwrap();
        println!(" - Trying environment var(LND_ADMINMACAROON): [{}]", helipad_config.macaroon_path);
    } else if server_config.macaroon.is_some() {
        helipad_config.macaroon_path = server_config.macaroon.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.macaroon_path);
    } else if Path::new("admin.macaroon").is_file() {
        helipad_config.macaroon_path = "admin.macaroon".to_string();
        println!(" - Trying current directory: [{}]", helipad_config.macaroon_path);
    } else {
        helipad_config.macaroon_path = String::from(LND_STANDARD_MACAROON_LOCATION);
        println!(" - Trying LND default: [{}]", helipad_config.macaroon_path);
    }

    println!("\nDiscovering certificate file path...");
    let env_cert_path = std::env::var("LND_TLSCERT");
    if env_cert_path.is_ok() {
        helipad_config.cert_path = env_cert_path.unwrap();
        println!(" - Trying environment var(LND_TLSCERT): [{}]", helipad_config.cert_path);
    } else if server_config.cert.is_some() {
        helipad_config.cert_path = server_config.cert.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.cert_path);
    } else if Path::new("tls.cert").is_file() {
        helipad_config.cert_path = "tls.cert".to_string();
        println!(" - Trying current directory: [{}]", helipad_config.cert_path);
    } else {
        helipad_config.cert_path = String::from(LND_STANDARD_TLSCERT_LOCATION);
        println!(" - Trying LND default: [{}]", helipad_config.cert_path);
    }

    //Get the url connection string of the lnd node
    println!("\nDiscovering LND node address...");
    let env_lnd_url = std::env::var("LND_URL");
    if env_lnd_url.is_ok() {
        helipad_config.node_address = "https://".to_owned() + env_lnd_url.unwrap().as_str();
        println!(" - Trying environment var(LND_URL): [{}]", helipad_config.node_address);
    } else if server_config.lnd_url.is_some() {
        helipad_config.node_address = server_config.lnd_url.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, helipad_config.node_address);
    } else {
        helipad_config.node_address = String::from(LND_STANDARD_GRPC_URL);
        println!(" - Trying localhost default: [{}].", helipad_config.node_address);
    }

    //Load initial settings from database
    let initial_settings = match dbif::load_settings_from_db(&helipad_config.database_file_path) {
        Ok(settings) => settings,
        Err(e) => {
            eprintln!("Error loading initial settings: {:#?}", e);
            dbif::SettingsRecord {
                show_received_sats: false,
                show_split_percentage: false,
                hide_boosts: false,
                hide_boosts_below: None,
                play_pew: true,
                custom_pew_file: None,
                resolve_nostr_refs: false,
                show_hosted_wallet_ids: false,
                show_lightning_invoices: true,
                fetch_metadata: true,
            }
        }
    };
    let shared_settings = Arc::new(RwLock::new(initial_settings));

    //App State
    let state = AppState {
        helipad_config: helipad_config.clone(),
        version: version.to_string(),
        ws_tx: Arc::new(broadcast::Sender::new(100)),
        settings: shared_settings.clone(),
    };

    //Start the LND polling thread.  This thread will poll LND every few seconds to
    //get the latest invoices and store them in the database.
    tokio::spawn(poller::lnd_poller(helipad_config.clone(), state.ws_tx.clone()));
    tokio::spawn(poller::lnd_subscribe_invoices(helipad_config.clone(), state.ws_tx.clone(), shared_settings.clone()));

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

            .route("/settings/triggers", get(handler::trigger_settings_list))
            .route("/settings/triggers/:idx", patch(handler::trigger_settings_patch))
            .route("/settings/triggers/:idx", get(handler::trigger_settings_load))
            .route("/settings/triggers/:idx", post(handler::trigger_settings_save))
            .route("/settings/triggers/:idx", delete(handler::trigger_settings_delete))
            .route("/settings/triggers/:idx/test", post(handler::trigger_settings_test))

            .route("/settings/report/podcasts", get(handler::report_podcasts_list))
            .route("/settings/report/generate", post(handler::report_generate))

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
                .route("/podcasts", get(handler::api_v1_podcasts))
                .route("/sent_podcasts", get(handler::api_v1_sent_podcasts))
                .route("/ws", any(websocket_handler))

                // allow all origins to GET from public api
                .route_layer(CorsLayer::new().allow_methods([Method::GET]).allow_origin(Any))
            )

            // protected api
            .route("/api/v1/reply", post(handler::api_v1_reply))
            .route("/api/v1/mark_replied", post(handler::api_v1_mark_replied))
            .route("/api/v1/fetch_metadata/:idx", post(handler::api_v1_fetch_metadata))

            // require auth for above routes
            .route_layer(middleware::from_fn_with_state(state.clone(), handler::auth_middleware))
        )

        // login page
        .route("/login", get(handler::login).post(handler::handle_login))

        // api login endpoint
        .route("/api/v1/login", post(handler::api_v1_login))

        // static assets
        .nest_service("/image", ServeDir::new(WEBROOT_PATH_IMAGE))
        .nest_service("/script", ServeDir::new(WEBROOT_PATH_SCRIPT))
        .nest_service("/style", ServeDir::new(WEBROOT_PATH_STYLE))
        .nest_service(
            "/sound",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    header::CACHE_CONTROL,
                    AxumHeaderValue::from_static("no-cache")
                ))
                .service(ServeDir::new(helipad_config.sound_path))
        )

        .nest_service("/pew.mp3", ServeFile::new("webroot/extra/pew.mp3"))
        .nest_service("/favicon.ico", ServeFile::new("webroot/extra/favicon.ico"))
        .nest_service("/apps.json", ServeFile::new("webroot/extra/apps.json"))

        .with_state(state);


    let binding = format!("0.0.0.0:{}", &listen_port);
    let listener = tokio::net::TcpListener::bind(&binding).await.unwrap();

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

    println!("\nHelipad is listening on http://{}", binding);
    axum::serve(listener, app).await.unwrap();
}


async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

async fn handle_websocket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.ws_tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        let json = serde_json::to_string(&msg).unwrap();
        if let Err(e) = socket.send(json.into()).await {
            eprintln!("Error sending message to WebSocket: {}", e);
            break;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebSocketEvent(
    pub String,
    pub serde_json::Value,
);
