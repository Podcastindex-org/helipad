//Modules ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use route_recognizer::Params;
use router::Router;
use std::sync::Arc;
use hyper::server::conn::AddrStream;
use std::env;
use drop_root::set_user_group;

use std::path::Path;
use rand::{distributions::Alphanumeric, Rng}; // 0.8

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT, HeaderMap, HeaderValue};
use reqwest::redirect::Policy;

use chrono::Utc;

#[macro_use]
extern crate configure_me;


//Globals ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
mod cookies;
mod handler;
mod router;
mod lightning;
mod podcastindex;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

const HELIPAD_CONFIG_FILE: &str = "./helipad.conf";
const HELIPAD_DATABASE_DIR: &str = "database.db";
const HELIPAD_STANDARD_PORT: &str = "2112";

const LND_STANDARD_GRPC_URL: &str = "https://127.0.0.1:10009";
const LND_STANDARD_MACAROON_LOCATION: &str = "/lnd/data/chain/bitcoin/mainnet/admin.macaroon";
const LND_STANDARD_TLSCERT_LOCATION: &str = "/lnd/tls.cert";

const REMOTE_GUID_CACHE_SIZE: usize = 20;

//Structs ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct AppState {
    pub state_thing: String,
    pub remote_ip: String,
    pub version: String,
}

#[derive(Clone, Debug)]
pub struct HelipadConfig {
    pub database_file_path: String,
    pub listen_port: String,
    pub macaroon_path: String,
    pub cert_path: String,
    pub node_address: String,
    pub password: String,
    pub secret: String,
}

#[derive(Debug)]
pub struct Context {
    pub state: AppState,
    pub req: Request<Body>,
    pub path: String,
    pub params: Params,
    pub helipad_config: HelipadConfig,
    body_bytes: Option<hyper::body::Bytes>,
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

    //Start the LND polling thread.  This thread will poll LND every few seconds to
    //get the latest invoices and store them in the database.
    tokio::spawn(lnd_poller(helipad_config.clone()));

    //Router
    let some_state = "state".to_string();
    let mut router: Router = Router::new();

    //Base
    router.get("/", Box::new(handler::home));
    router.get("/login", Box::new(handler::login));
    router.post("/login", Box::new(handler::login));
    router.get("/streams", Box::new(handler::streams));
    router.get("/sent", Box::new(handler::sent));
    router.get("/settings", Box::new(handler::settings));
    router.get("/pew.mp3", Box::new(handler::pewmp3));
    router.get("/favicon.ico", Box::new(handler::favicon));
    router.get("/apps.json", Box::new(handler::apps_json));
    router.get("/numerology.json", Box::new(handler::numerology_json));
    //Assets
    router.get("/image", Box::new(handler::asset));
    router.get("/html", Box::new(handler::asset));
    router.get("/style", Box::new(handler::asset));
    router.get("/script", Box::new(handler::asset));
    router.get("/extra", Box::new(handler::asset));
    //Api
    router.options("/api/v1/node_info", Box::new(handler::api_v1_node_info_options));
    router.get("/api/v1/node_info", Box::new(handler::api_v1_node_info));
    router.options("/api/v1/boosts", Box::new(handler::api_v1_boosts_options));
    router.get("/api/v1/boosts", Box::new(handler::api_v1_boosts));
    router.options("/api/v1/balance", Box::new(handler::api_v1_balance_options));
    router.get("/api/v1/balance", Box::new(handler::api_v1_balance));
    router.options("/api/v1/streams", Box::new(handler::api_v1_streams_options));
    router.get("/api/v1/streams", Box::new(handler::api_v1_streams));
    router.options("/api/v1/sent", Box::new(handler::api_v1_sent_options));
    router.get("/api/v1/sent", Box::new(handler::api_v1_sent));
    router.options("/api/v1/index", Box::new(handler::api_v1_index_options));
    router.get("/api/v1/index", Box::new(handler::api_v1_index));
    router.options("/api/v1/sent_index", Box::new(handler::api_v1_sent_index_options));
    router.get("/api/v1/sent_index", Box::new(handler::api_v1_sent_index));
    router.options("/api/v1/reply", Box::new(handler::api_v1_reply_options));
    router.post("/api/v1/reply", Box::new(handler::api_v1_reply));
    router.post("/api/v1/mark_replied", Box::new(handler::api_v1_mark_replied));
    router.get("/api/v1/webhooks", Box::new(handler::api_v1_webhooks));
    router.get("/api/v1/webhooks/:idx", Box::new(handler::api_v1_webhook_edit));
    router.post("/api/v1/webhooks/:idx", Box::new(handler::api_v1_webhook_save));
    router.delete("/api/v1/webhooks/:idx", Box::new(handler::api_v1_webhook_delete));
    router.get("/csv", Box::new(handler::csv_export_boosts));


    let shared_router = Arc::new(router);
    let hp_config = helipad_config.clone();
    let new_service = make_service_fn(move |conn: &AddrStream| {
        let app_state = AppState {
            state_thing: some_state.clone(),
            remote_ip: conn.remote_addr().to_string().clone(),
            version: version.to_string(),
        };

        let helipad_config = hp_config.clone();
        let router_capture = shared_router.clone();
        async {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, app_state.clone(), helipad_config.clone())
            }))
        }
    });

    let binding = format!("0.0.0.0:{}", &listen_port);
    let addr = binding.parse().expect("address creation works");
    let server = Server::bind(&addr).serve(new_service);
    println!("\nHelipad is listening on http://{}", addr);

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

    let _ = server.await;
}

async fn route(
    router: Arc<Router>,
    req: Request<hyper::Body>,
    app_state: AppState,
    helipad_config: HelipadConfig,
) -> Result<Response, Error> {
    let found_handler = router.route(req.uri().path(), req.method());
    let path = req.uri().path().to_owned();
    let ctx = Context::new(app_state, req, &path, found_handler.params, helipad_config.clone());

    if let Some(resp) = handler::login_required(&ctx) {
        return Ok(resp);
    }

    let authed = handler::verify_jwt_cookie(&ctx.req, &helipad_config.secret);

    let mut resp = found_handler
        .handler
        .invoke(ctx)
        .await;

    if authed {
        handler::set_jwt_cookie(&mut resp, &helipad_config.secret);
    }

    Ok(resp)
}

impl Context {
    pub fn new(state: AppState, reqbody: Request<Body>, path: &str, params: Params, helipad_config: HelipadConfig) -> Context {
        Context {
            state: state,
            req: reqbody,
            path: path.to_string(),
            params: params,
            helipad_config: helipad_config,
            body_bytes: None,
        }
    }

    pub async fn body_json<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, Error> {
        let body_bytes = match self.body_bytes {
            Some(ref v) => v,
            _ => {
                let body = to_bytes(self.req.body_mut()).await?;
                self.body_bytes = Some(body);
                self.body_bytes.as_ref().expect("body_bytes was set above")
            }
        };
        Ok(serde_json::from_slice(&body_bytes)?)
    }
}

//The LND poller runs in a thread and pulls new invoices
async fn lnd_poller(helipad_config: HelipadConfig) {
    let db_filepath = helipad_config.database_file_path.clone();

    //Make the connection to LND
    println!("\nConnecting to LND node address...");
    let mut lightning;
    match lightning::connect_to_lnd(helipad_config.node_address, helipad_config.cert_path, helipad_config.macaroon_path).await {
        Some(lndconn) => {
            println!(" - Success.");
            lightning = lndconn;
        }
        None => {
            std::process::exit(1);
        }
    }

    //Get lnd node info
    match lnd::Lnd::get_info(&mut lightning).await {
        Ok(node_info) => {
            println!("LND node info: {:#?}", node_info);

            let record = dbif::NodeInfoRecord {
                lnd_alias: node_info.alias,
                node_pubkey: node_info.identity_pubkey,
                node_version: node_info.version,
            };

            if dbif::add_node_info_to_db(&db_filepath, record).is_err() {
                println!("Error updating node info in database.");
            }
        }
        Err(e) => {
            eprintln!("Error getting LND node info: {:#?}", e);
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
        match lnd::Lnd::channel_balance(&mut lightning).await {
            Ok(balance) => {
                let mut current_balance: i64 = 0;
                if let Some(bal) = balance.local_balance {
                    println!("LND node local balance: {:#?}", bal.sat);
                    current_balance = bal.sat as i64;
                }

                if dbif::add_wallet_balance_to_db(&db_filepath, current_balance).is_err() {
                    println!("Error adding wallet balance to the database.");
                }
            }
            Err(e) => {
                eprintln!("Error getting LND wallet balance: {:#?}", e);
            }
        }

        //Get a list of invoices
        match lnd::Lnd::list_invoices(&mut lightning, false, current_index.clone(), 500, false).await {
            Ok(response) => {
                for invoice in response.invoices {
                    let parsed = lightning::parse_boost_from_invoice(invoice.clone(), &mut remote_cache).await;

                    if let Some(boost) = parsed {
                        //Give some output
                        println!("Boost: {:#?}", &boost);

                        //Store in the database
                        match dbif::add_invoice_to_db(&db_filepath, &boost) {
                            Ok(_) => println!("New invoice added."),
                            Err(e) => eprintln!("Error adding invoice: {:#?}", e)
                        }

                        //Send out webhooks (if any)
                        send_webhooks(&db_filepath, &boost).await;
                    }

                    current_index = invoice.add_index;
                    updated = true;
                }
            }
            Err(e) => {
                eprintln!("lnd::Lnd::list_invoices failed: {}", e);
            }
        }

        //Make sure we are tracking our position properly
        println!("Current index: {}", current_index);

        match lnd::Lnd::list_payments(&mut lightning, false, current_payment, 500, false).await {
            Ok(response) => {
                for payment in response.payments {
                    let parsed = lightning::parse_boost_from_payment(payment.clone(), &mut remote_cache).await;

                    if let Some(boost) = parsed {
                        //Give some output
                        println!("Sent Boost: {:#?}", boost);

                        //Store in the database
                        match dbif::add_payment_to_db(&db_filepath, &boost) {
                            Ok(_) => println!("New payment added."),
                            Err(e) => eprintln!("Error adding payment: {:#?}", e)
                        }
                    }

                    current_payment = payment.payment_index;
                    updated = true;
                }
            }
            Err(e) => {
                eprintln!("lnd::Lnd::list_payments failed: {}", e);
            }
        };

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
        Err(_) => {
            return;
        }
    };

    for webhook in webhooks {
        let mut headers = HeaderMap::new();

        headers.insert(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap());

        let user_agent = format!("Helipad/{}", env!("CARGO_PKG_VERSION"));
        headers.insert(USER_AGENT, HeaderValue::from_str(user_agent.as_str()).unwrap());

        if webhook.token != "" {
            let token = format!("Bearer {}", webhook.token);
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&token).unwrap());
        }

        let client = reqwest::Client::builder()
            .redirect(Policy::limited(5))
            .build()
            .unwrap();

        let json = serde_json::to_string_pretty(&boost).unwrap();
        let response = client
            .post(&webhook.url)
            .body(json)
            .headers(headers)
            .send()
            .await
            .unwrap()
            .text()
            .await;

        let timestamp = Utc::now().timestamp();
        let successful = response.is_ok();

        match response {
            Ok(resp) => println!("Webhook sent to {}: {}", webhook.url, resp),
            Err(e) => eprintln!("Webhook Error: {}", e),
        };

        if let Err(e) = dbif::set_webhook_last_request(&db_filepath, webhook.index, successful, timestamp) {
            eprintln!("Error setting webhook last request status: {}", e);
        }
    }
}