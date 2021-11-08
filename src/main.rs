use hyper::{
    body::to_bytes,
    service::{make_service_fn, service_fn},
    Body, Request, Server,
};
use route_recognizer::Params;
use router::Router;
use std::sync::Arc;
use hyper::server::conn::AddrStream;
use std::thread;
//use std::time;
//use tokio::task;
use std::fs;
use std::env;
use drop_root::set_user_group;
use lnd;
use serde::{Deserialize, Serialize};

//Globals ----------------------------------------------------------------------------------------------------

mod handler;
mod router;

type Response = hyper::Response<hyper::Body>;
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;


//Structs ----------------------------------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct AppState {
    pub state_thing: String,
    pub remote_ip: String,
}

#[derive(Debug)]
pub struct Context {
    pub state: AppState,
    pub req: Request<Body>,
    pub params: Params,
    body_bytes: Option<hyper::body::Bytes>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
struct RawBoost {
    action: Option<String>,
    app_name: Option<String>,
    app_version: Option<String>,
    boost_link: Option<String>,
    message: Option<String>,
    name: Option<String>,
    pubkey: Option<String>,
    sender_key: Option<String>,
    sender_name: Option<String>,
    sender_id: Option<String>,
    sig_fields: Option<String>,
    signature: Option<String>,
    speed: Option<String>,
    uuid: Option<String>,
    podcast: Option<String>,
    feedID: Option<u64>,
    guid: Option<String>,
    url: Option<String>,
    episode: Option<String>,
    itemID: Option<u64>,
    episode_guid: Option<String>,
    time: Option<String>,
    ts: Option<u64>,
    value_msat: Option<u64>,
    value_msat_total: Option<u64>,
}

//Functions --------------------------------------------------------------------------------------------------
#[tokio::main]
async fn main() {

    //Get command line args
    let args: Vec<String> = env::args().collect();
    let arg_port = &args[1];

    //Create a new database if needed
    dbif::create_database();

    //LND polling thread
    tokio::spawn(lnd_poller());

    //Router
    let some_state = "state".to_string();
    let mut router: Router = Router::new();
    router.get("/", Box::new(handler::home));
    router.get("/home.js", Box::new(handler::homejs));
    router.get("/pew.mp3", Box::new(handler::pewmp3));
    router.get("/utils.js", Box::new(handler::utilsjs));
    router.get("/boosts", Box::new(handler::boosts));

    let shared_router = Arc::new(router);
    let new_service = make_service_fn(move |conn: &AddrStream| {
        let app_state = AppState {
            state_thing: some_state.clone(),
            remote_ip: conn.remote_addr().to_string().clone(),
        };

        let router_capture = shared_router.clone();
        async {
            Ok::<_, Error>(service_fn(move |req| {
                route(router_capture.clone(), req, app_state.clone())
            }))
        }
    });

    let binding = format!("0.0.0.0:{}", arg_port);
    let addr = binding.parse().expect("address creation works");
    let server = Server::bind(&addr).serve(new_service);
    println!("Listening on http://{}", addr);

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
) -> Result<Response, Error> {
    let found_handler = router.route(req.uri().path(), req.method());
    let resp = found_handler
        .handler
        .invoke(Context::new(app_state, req, found_handler.params))
        .await;
    Ok(resp)
}

impl Context {
    pub fn new(state: AppState, req: Request<Body>, params: Params) -> Context {
        Context {
            state,
            req,
            params,
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
async fn lnd_poller() {

    //Get the macaroon and cert files
    let macaroon = fs::read("admin.macaroon").unwrap();
    let cert = fs::read("tls.cert").unwrap();

    //Create the database if needed
    match dbif::create_database() {
        Ok(_) => {
            println!("Created new database.");
        }
        Err(e) => {
            println!("Could not create database file: {}", dbif::SQLITE_FILE);
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }


    //Get the url connection string of the lnd node
    let env_lnd_url = std::env::var("LND_URL");
    if env_lnd_url.is_err() {
        println!("The $LND_URL environment variable could not be found.\n");
        std::process::exit(1);
    }
    let node_address = "https://".to_owned() + env_lnd_url.unwrap().as_str();

    //Make the connection to LND
    let mut lightning;
    match lnd::Lnd::connect_with_macaroon(node_address.clone(), &cert, &macaroon).await {
        Ok(lndconn) => {
            lightning = lndconn;
        }
        Err(e) => {
            println!("Could not connect to: {} using tls and macaroon", node_address);
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    //The main loop
    let mut current_index = dbif::get_last_boost_index_from_db().unwrap();
    loop {

        //Get a list of invoices
        match lnd::Lnd::list_invoices(&mut lightning, false, current_index.clone(), 100, false).await {
            Ok(response) => {
                for invoice in response.invoices {

                    //Initialize a boost record
                    let mut boost = dbif::BoostRecord {
                        index: invoice.add_index,
                        time: invoice.settle_date,
                        value_msat: invoice.amt_paid_sat,
                        message: "".to_string(),
                        tlv: "".to_string(),
                    };

                    //Search for podcast boost tlvs
                    for htlc in invoice.htlcs {
                        for (idx, val) in htlc.custom_records {
                            //Satoshis.stream record type
                            if idx == 7629169 {
                                boost.tlv = std::str::from_utf8(&val).unwrap().to_string();
                                let tlv = std::str::from_utf8(&val).unwrap();
                                let json_result = serde_json::from_str::<RawBoost>(tlv);
                                match json_result {
                                    Ok(rawboost) => {
                                        println!("{:#?}", rawboost);
                                        if rawboost.message.is_some() {
                                            boost.message = rawboost.message.unwrap();
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("{}", e);
                                    }
                                }
                            }
                        }
                    }

                    //Give some output
                    println!("Boost: {:#?}", boost);

                    //Store in the database
                    dbif::add_invoice_to_db(boost);
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }

        //Make sure we are tracking our position properly
        current_index = dbif::get_last_boost_index_from_db().unwrap();
        println!("Current index: {}", current_index);

        std::thread::sleep(std::time::Duration::from_millis(9000));
    }
}