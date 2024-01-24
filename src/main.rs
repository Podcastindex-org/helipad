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
use std::fs;
use std::env;
use drop_root::set_user_group;
use lnd;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use dbif::add_wallet_balance_to_db;

// use hyper::http::Request;
use reqwest;
use reqwest::header::USER_AGENT;
use lru::LruCache;
use std::num::NonZeroUsize;

#[macro_use]
extern crate configure_me;


//Globals ----------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
mod handler;
mod router;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

const HELIPAD_CONFIG_FILE: &str = "./helipad.conf";
const HELIPAD_DATABASE_DIR: &str = "database.db";
const HELIPAD_STANDARD_PORT: &str = "2112";
const LND_STANDARD_GRPC_URL: &str = "https://127.0.0.1:10009";
const LND_STANDARD_MACAROON_LOCATION: &str = "/lnd/data/chain/bitcoin/mainnet/admin.macaroon";
const LND_STANDARD_TLSCERT_LOCATION: &str = "/lnd/tls.cert";
const REMOTE_GUID_CACHE_SIZE: usize = 20;

// TLV keys (see https://github.com/satoshisstream/satoshis.stream/blob/main/TLV_registry.md)
const TLV_PODCASTING20: u64 = 7629169;
const TLV_WALLET_KEY: u64 = 696969;
const TLV_WALLET_ID: u64 = 112111100;
const TLV_HIVE_ACCOUNT: u64 = 818818;

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
}

#[derive(Debug)]
pub struct Context {
    pub state: AppState,
    pub req: Request<Body>,
    pub path: String,
    pub params: Params,
    pub database_file_path: String,
    body_bytes: Option<hyper::body::Bytes>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct RawBoost {
    #[serde(default = "d_action")]
    action: Option<String>,
    #[serde(default = "d_blank")]
    app_name: Option<String>,
    #[serde(default = "d_blank")]
    app_version: Option<String>,
    #[serde(default = "d_blank")]
    boost_link: Option<String>,
    #[serde(default = "d_blank")]
    message: Option<String>,
    #[serde(default = "d_blank")]
    name: Option<String>,
    #[serde(default = "d_blank")]
    pubkey: Option<String>,
    #[serde(default = "d_blank")]
    sender_key: Option<String>,
    #[serde(default = "d_blank")]
    sender_name: Option<String>,
    #[serde(default = "d_blank")]
    sender_id: Option<String>,
    #[serde(default = "d_blank")]
    sig_fields: Option<String>,
    #[serde(default = "d_blank")]
    signature: Option<String>,
    #[serde(default = "d_blank")]
    speed: Option<String>,
    #[serde(default = "d_blank")]
    uuid: Option<String>,
    #[serde(default = "d_blank")]
    podcast: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    feedID: Option<u64>,
    #[serde(default = "d_blank")]
    guid: Option<String>,
    #[serde(default = "d_blank")]
    url: Option<String>,
    #[serde(default = "d_blank")]
    episode: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    itemID: Option<u64>,
    #[serde(default = "d_blank")]
    episode_guid: Option<String>,
    #[serde(default = "d_blank")]
    time: Option<String>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    ts: Option<u64>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat: Option<u64>,
    #[serde(default = "d_zero", deserialize_with = "de_optional_string_or_number")]
    value_msat_total: Option<u64>,
    #[serde(default = "d_blank")]
    remote_feed_guid: Option<String>,
    #[serde(default = "d_blank")]
    remote_item_guid: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PodcastEpisodeGuid {
    pub podcast_guid: String,
    pub episode_guid: String,
    pub podcast: String,
    pub episode: String,
}


//Traits------------------------------------------------------------------------------------------------------
//------------------------------------------------------------------------------------------------------------
fn d_action() -> Option<String> {
    Some("stream".to_string())
}

fn d_blank() -> Option<String> {
    None
}

fn d_zero() -> Option<u64> {
    None
}

fn de_optional_string_or_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u64>, D::Error> {
    Ok(
        match Value::deserialize(deserializer)? {
            Value::String(s) => {
                if s.is_empty() {
                    return Ok(None);
                }
                if let Ok(number) = s.parse() {
                    Some(number)
                } else {
                    return Ok(None);
                }
            }
            Value::Number(num) => {
                if num.is_u64() {
                    if let Some(number) = num.as_u64() {
                        Some(number)
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                }

            }
            _ => Some(0)
        }
    )
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

    //Start the LND polling thread.  This thread will poll LND every few seconds to
    //get the latest invoices and store them in the database.
    tokio::spawn(lnd_poller(server_config, helipad_config.database_file_path.clone()));

    //Router
    let some_state = "state".to_string();
    let mut router: Router = Router::new();

    //Base
    router.get("/", Box::new(handler::home));
    router.get("/streams", Box::new(handler::streams));
    router.get("/sent", Box::new(handler::sent));
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
    router.get("/csv", Box::new(handler::csv_export_boosts));


    let shared_router = Arc::new(router);
    let db_filepath: String = helipad_config.database_file_path.clone();
    let new_service = make_service_fn(move |conn: &AddrStream| {
        let app_state = AppState {
            state_thing: some_state.clone(),
            remote_ip: conn.remote_addr().to_string().clone(),
            version: version.to_string(),
        };

        let database_file_path = db_filepath.clone();

        let router_capture = shared_router.clone();
        async {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, app_state.clone(), database_file_path.clone())
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
    database_file_path: String,
) -> Result<Response, Error> {
    let found_handler = router.route(req.uri().path(), req.method());
    let path = req.uri().path().to_owned();
    let resp = found_handler
        .handler
        .invoke(Context::new(app_state, req, &path, found_handler.params, database_file_path))
        .await;
    Ok(resp)
}

impl Context {
    pub fn new(state: AppState, reqbody: Request<Body>, path: &str, params: Params, database_file_path: String) -> Context {
        Context {
            state: state,
            req: reqbody,
            path: path.to_string(),
            params: params,
            database_file_path: database_file_path,
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

// Fetches remote podcast/episode names by guids using the Podcastindex API and caches results into an LRU cache
pub async fn fetch_podcast_episode_by_guid(cache: &mut LruCache<String, Option<PodcastEpisodeGuid>>, podcast_guid: String, episode_guid: String) -> Option<PodcastEpisodeGuid> {
    let key = format!("{}_{}", podcast_guid, episode_guid);

    if let Some(cached_guid) = cache.get(&key) {
        println!("Remote podcast/episode from cache: {:#?}", cached_guid);
        return cached_guid.clone(); // already exists in cache
    }

    match fetch_api_podcast_episode_by_guid(&podcast_guid, &episode_guid).await {
        Ok(Some(guid)) => {
            println!("Remote podcast/episode from API: {:#?}", guid);
            cache.put(key, Some(guid.clone())); // cache to avoid spamming api
            Some(guid)
        },
        Ok(None) => {
            println!("Remote podcast/episode not found {} {}", podcast_guid, episode_guid);
            cache.put(key, None); // cache to avoid spamming api
            None
        }
        Err(e) => {
            eprintln!("Error retrieving remote podcast/episode from API: {:#?}", e);
            None
        }
    }
}

// Fetches remote podcast/episode names by guids using the Podcastindex API
pub async fn fetch_api_podcast_episode_by_guid(podcast_guid: &String, episode_guid: &String) -> Result<Option<PodcastEpisodeGuid>, Error> {
    let query = vec![
        ("podcastguid", podcast_guid),
        ("episodeguid", episode_guid)
    ];

    let app_version = env!("CARGO_PKG_VERSION");

    // call API, get text response, and parse into json
    let response = reqwest::Client::new()
        .get("https://api.podcastindex.org/api/1.0/value/byepisodeguid")
        .header(USER_AGENT, format!("Helipad/{}", app_version))
        .query(&query)
        .send()
        .await?;

    let result = response.text().await?;
    let json: Value = serde_json::from_str(&result)?;

    let status = json["status"].as_str().unwrap_or_default();

    if status != "true" {
        return Ok(None); // not found?
    }

    let query = match json["query"].as_object() {
        Some(val) => val,
        None => { return Ok(None); }
    };

    let value = match json["value"].as_object() {
        Some(val) => val,
        None => { return Ok(None); }
    };

    let found_podcast_guid = query["podcastguid"].as_str().unwrap_or_default();
    let found_episode_guid = query["episodeguid"].as_str().unwrap_or_default();

    let found_podcast = value["feedTitle"].as_str().unwrap_or_default();
    let found_episode = value["title"].as_str().unwrap_or_default();

    return Ok(Some(PodcastEpisodeGuid {
        podcast_guid: found_podcast_guid.to_string(),
        episode_guid: found_episode_guid.to_string(),
        podcast: found_podcast.to_string(),
        episode: found_episode.to_string(),
    }))
}

async fn parse_podcast_tlv(boost: &mut dbif::BoostRecord, val: &Vec<u8>, remote_cache: &mut LruCache<String, Option<PodcastEpisodeGuid>>) {
    let tlv = std::str::from_utf8(&val).unwrap();
    println!("TLV: {:#?}", tlv);

    boost.tlv = tlv.to_string();

    let json_result = serde_json::from_str::<RawBoost>(tlv);
    match json_result {
        Ok(rawboost) => {
            //If there was a sat value in the tlv, override the invoice
            if rawboost.value_msat.is_some() {
                boost.value_msat = rawboost.value_msat.unwrap() as i64;
            }

            //Determine an action type for later filtering ability
            if rawboost.action.is_some() {
                boost.action = match rawboost.action.unwrap().as_str() {
                    "stream" => 1, //This indicates a per-minute podcast payment
                    "boost" => 2,  //This is a manual boost or boost-a-gram
                    _ => 3,
                }
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
            if rawboost.remote_feed_guid.is_some() && rawboost.remote_item_guid.is_some() {
                let remote_feed_guid = rawboost.remote_feed_guid.unwrap();
                let remote_item_guid = rawboost.remote_item_guid.unwrap();

                let episode_guid = fetch_podcast_episode_by_guid(remote_cache, remote_feed_guid, remote_item_guid).await;

                if let Some(guid) = episode_guid {
                    boost.remote_podcast = Some(guid.podcast);
                    boost.remote_episode = Some(guid.episode);
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
        }
    }
}

//The LND poller runs in a thread and pulls new invoices
async fn lnd_poller(server_config: Config, database_file_path: String) {
    let db_filepath = database_file_path;

    //Get the macaroon and cert files.  Look in the local directory first as an override.
    //If the files are not found in the currect working directory, look for them at their
    //normal LND directory locations
    println!("\nDiscovering macaroon file path...");
    let macaroon_path;
    let env_macaroon_path = std::env::var("LND_ADMINMACAROON");
    //First try from the environment
    if env_macaroon_path.is_ok() {
        macaroon_path = env_macaroon_path.unwrap();
        println!(" - Trying environment var(LND_ADMINMACAROON): [{}]", macaroon_path);
    } else if server_config.macaroon.is_some() {
        macaroon_path = server_config.macaroon.unwrap();
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

    println!("\nDiscovering certificate file path...");
    let cert_path;
    let env_cert_path = std::env::var("LND_TLSCERT");
    if env_cert_path.is_ok() {
        cert_path = env_cert_path.unwrap();
        println!(" - Trying environment var(LND_TLSCERT): [{}]", cert_path);
    } else if server_config.cert.is_some() {
        cert_path = server_config.cert.unwrap();
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

    //Get the url connection string of the lnd node
    println!("\nDiscovering LND node address...");
    let node_address;
    let env_lnd_url = std::env::var("LND_URL");
    if env_lnd_url.is_ok() {
        node_address = "https://".to_owned() + env_lnd_url.unwrap().as_str();
        println!(" - Trying environment var(LND_URL): [{}]", node_address);
    } else if server_config.lnd_url.is_some() {
        node_address = server_config.lnd_url.unwrap();
        println!(" - Trying config file({}): [{}]", HELIPAD_CONFIG_FILE, node_address);
    } else {
        node_address = String::from(LND_STANDARD_GRPC_URL);
        println!(" - Trying localhost default: [{}].", node_address);
    }

    //Make the connection to LND
    let mut lightning;
    match lnd::Lnd::connect_with_macaroon(node_address.clone(), &cert, &macaroon).await {
        Ok(lndconn) => {
            println!(" - Success.");
            lightning = lndconn;
        }
        Err(e) => {
            println!("Could not connect to: [{}] using tls: [{}] and macaroon: [{}]", node_address, cert_path, macaroon_path);
            eprintln!("{:#?}", e);
            std::process::exit(1);
        }
    }

    //Get lnd node info
    match lnd::Lnd::get_info(&mut lightning).await {
        Ok(node_info) => {
            println!("LND node info: {:#?}", node_info);
        }
        Err(e) => {
            eprintln!("Error getting LND node info: {:#?}", e);
        }
    }

    //Instantiate a cache to use when resolving remote podcasts/episode guids
    let mut remote_cache = LruCache::new(NonZeroUsize::new(REMOTE_GUID_CACHE_SIZE).unwrap());

    //The main loop
    let mut current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();
    let mut current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();
    loop {

        //Get lnd node channel balance
        match lnd::Lnd::channel_balance(&mut lightning).await {
            Ok(balance) => {
                let mut current_balance: i64 = 0;
                if let Some(bal) = balance.local_balance {
                    println!("LND node local balance: {:#?}", bal.sat);
                    current_balance = bal.sat as i64;
                }

                if add_wallet_balance_to_db(&db_filepath, current_balance).is_err() {
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
                        payment_info: None,
                    };

                    //Search for podcast boost tlvs
                    for htlc in invoice.htlcs {
                        for (idx, val) in htlc.custom_records {
                            //Satoshis.stream record type
                            if idx == TLV_PODCASTING20 {
                                parse_podcast_tlv(&mut boost, &val, &mut remote_cache).await;
                            }
                        }
                    }

                    //Give some output
                    println!("Boost: {:#?}", boost);

                    //Store in the database
                    println!("{:#?}", boost);
                    match dbif::add_invoice_to_db(&db_filepath, boost) {
                        Ok(_) => println!("New invoice added."),
                        Err(e) => eprintln!("Error adding invoice: {:#?}", e)
                    }
                }
            }
            Err(e) => {
                eprintln!("lnd::Lnd::list_invoices failed: {}", e);
            }
        }

        //Make sure we are tracking our position properly
        current_index = dbif::get_last_boost_index_from_db(&db_filepath).unwrap();
        println!("Current index: {}", current_index);

        match lnd::Lnd::list_payments(&mut lightning, false, current_payment, 500, false).await {
            Ok(response) => {
                for payment in response.payments {

                    for htlc in payment.htlcs {

                        if let Some(route) = htlc.route {
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
                                payment_info: Some(dbif::PaymentRecord {
                                    pubkey: hop.pub_key.clone(),
                                    custom_key: 0,
                                    custom_value: "".into(),
                                    fee_msat: payment.fee_msat,
                                }),
                            };

                            for (idx, val) in hop.custom_records {
                                if idx == TLV_PODCASTING20 {
                                    parse_podcast_tlv(&mut boost, &val, &mut remote_cache).await;
                                }
                                else if idx == TLV_WALLET_KEY || idx == TLV_WALLET_ID || idx == TLV_HIVE_ACCOUNT {
                                    let custom_value = std::str::from_utf8(&val).unwrap().to_string();

                                    boost.payment_info = Some(dbif::PaymentRecord {
                                        pubkey: hop.pub_key.clone(),
                                        custom_key: idx,
                                        custom_value: custom_value,
                                        fee_msat: payment.fee_msat,
                                    });
                                }
                            }

                            //Give some output
                            println!("Sent Boost: {:#?}", boost);

                            match dbif::add_payment_to_db(&db_filepath, boost) {
                                Ok(_) => println!("New payment added."),
                                Err(e) => eprintln!("Error adding payment: {:#?}", e)
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("lnd::Lnd::list_payments failed: {}", e);
            }
        };

        //Make sure we are tracking our position properly
        current_payment = dbif::get_last_payment_index_from_db(&db_filepath).unwrap();
        println!("Current payment: {}", current_payment);

        tokio::time::sleep(tokio::time::Duration::from_millis(9000)).await;
    }
}