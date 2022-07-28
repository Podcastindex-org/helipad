use crate::{Context, Response};
use hyper::StatusCode;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use voca_rs::*;
use handlebars::Handlebars;
use serde_json::json;
use chrono::{NaiveDateTime};
use dbif::BoostRecord;


//Constants --------------------------------------------------------------------------------------------------
const WEBROOT_PATH: &str = "webroot";


//Structs and Enums ------------------------------------------------------------------------------------------
#[derive(Debug)]
struct HydraError(String);

impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}

impl Error for HydraError {}


//Route handlers ---------------------------------------------------------------------------------------------

//Homepage html
pub async fn home(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/home.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Streams html
pub async fn streams(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/streams.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Serve a web asset by name from webroot subfolder according to it's requested type
pub async fn asset(ctx: Context) -> Response {
    println!("** Context: {:#?}", ctx);

    //Resolve the full webroot path and make sure it exists
    let base_path = fs::canonicalize(WEBROOT_PATH).unwrap();

    //Resolve the full path to the requested file and check if it exists
    let file_path = match fs::canonicalize(
        format!("{}/{}", WEBROOT_PATH, ctx.path.as_str())
    ) {
        Ok(path) => path,
        Err(_) => {
            return hyper::Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("NOT FOUND".into())
                .unwrap()
        }
    };

    //Paranoia check: Make sure the requested path is inside the webroot path
    if ! file_path.starts_with(base_path) {
        return hyper::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("NOT FOUND".into())
            .unwrap()
    }

    //Use the file extension to figure out the mime type
    let file_extension = match file_path.extension() {
        Some(ext) => ext.to_str().unwrap(),
        None => "", // file with no extension
    };

    let content_type = match file_extension {
        "html" => "text/html",
        "mp3"  => "audio/mpeg",
        "png"  => "image/png",
        "css"  => "text/css",
        "js"   => "text/javascript",
        "ico"  => "image/x-icon",
        _      => "text/plain", // unknown extension => plain text
    };

    //Attempt to serve the file
    if file_path.exists() {
        let file_to_serve = file_path.to_str().unwrap();
        println!("** Serving file: [{}]", file_to_serve);
        let file = fs::read(file_to_serve).expect("Something went wrong reading the file.");
        return hyper::Response::builder()
            .status(StatusCode::OK)
            .header("Content-type", content_type)
            .body(hyper::Body::from(file))
            .unwrap();
    } else {
        return hyper::Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("NOT FOUND".into())
            .unwrap()
    }
}

//API - give back the node balance
pub async fn api_v1_balance_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_balance(_ctx: Context) -> Response {
    //Get query parameters
    let _params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Get the boosts from db for returning
    match dbif::get_wallet_balance_from_db(&_ctx.database_file_path) {
        Ok(balance) => {
            let json_doc = serde_json::to_string_pretty(&balance).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting balance: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting balance.").into())
                .unwrap();
        }
    }
}

//API - serve boosts as JSON either in ascending or descending order
pub async fn api_v1_boosts_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_boosts(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied boostcount from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(&_ctx.database_file_path, index, boostcount, old, true) {
        Ok(boosts) => {
            let json_doc = serde_json::to_string_pretty(&boosts).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting boosts.").into())
                .unwrap();
        }
    }
}

//API - serve streams as JSON either in ascending or descending order
pub async fn api_v1_streams_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_streams(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting streams: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting streams: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied stream count from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting streams: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting streams: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Get the boosts from db for returning
    match dbif::get_streams_from_db(&_ctx.database_file_path, index, boostcount, old) {
        Ok(streams) => {
            let json_doc_raw = serde_json::to_string_pretty(&streams).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting streams: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting streams.").into())
                .unwrap();
        }
    }
}

//API - get the current invoice index number
pub async fn api_v1_index_options(_ctx: Context) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::from_u16(204).unwrap())
        .header("Access-Control-Allow-Methods", "GET, OPTIONS")
        .body(format!("").into())
        .unwrap();
}

pub async fn api_v1_index(_ctx: Context) -> Response {

    //Get the last known invoice index from the database
    match dbif::get_last_boost_index_from_db(&_ctx.database_file_path) {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            let json_doc_raw = serde_json::to_string_pretty(&index).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting current db index.").into())
                .unwrap();
        }
    };
}

//CSV export - max is 200 for now so the csv content can be built in memory
pub async fn csv_export_boosts(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'index' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied boostcount from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        }
        None => {
            eprintln!("** Error getting boosts: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => {}
    };

    //Was a stop index given?
    let mut endex: u64 = 0;
    match params.get("end") {
        Some(endexnum) => {
            endex = match endexnum.parse::<u64>() {
                Ok(endex) => {
                    println!("** Supplied endex from call: [{}]", endex);
                    endex
                }
                Err(_) => {
                    eprintln!("** Error getting boosts: 'endex' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'endex' parameter must be an integer.").into())
                        .unwrap();
                }
            };
        }
        None => {}
    };

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(&_ctx.database_file_path, index, boostcount, old, false) {
        Ok(boosts) => {
            let mut csv = String::new();

            //CSV column name header
            csv.push_str(format!("count, index, time, value_msat, value_sat, value_msat_total, value_sat_total, action, sender, app, message, podcast, episode\n").as_str());

            //Iterate the boost set
            let mut count: u64 = 1;
            for boost in boosts {
                //Parse out a friendly date
                let dt = NaiveDateTime::from_timestamp(boost.time, 0);
                let boost_time = dt.format("%e %b %Y %H:%M:%S UTC").to_string();

                //Translate to sats
                let mut value_sat = 0;
                if boost.value_msat > 1000 {
                    value_sat = boost.value_msat / 1000;
                }
                let mut value_sat_total = 0;
                if boost.value_msat_total > 1000 {
                    value_sat_total = boost.value_msat_total / 1000;
                }

                //The main export data formatting
                csv.push_str(
                    format!(
                        "{},{},\"{}\",{},{},{},{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        count,
                        boost.index,
                        boost_time,
                        boost.value_msat,
                        value_sat,
                        boost.value_msat_total,
                        value_sat_total,
                        boost.action,
                        BoostRecord::escape_for_csv(boost.sender),
                        BoostRecord::escape_for_csv(boost.app),
                        BoostRecord::escape_for_csv(boost.message),
                        BoostRecord::escape_for_csv(boost.podcast),
                        BoostRecord::escape_for_csv(boost.episode)
                    ).as_str()
                );

                //Keep count
                count += 1;

                //If an exit point was given then bail when it's reached
                if (old && boost.index <= endex) || (!old && boost.index >= endex) {
                    break;
                }
            }

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-type", "text/plain; charset=utf-8")
                .header("Content-Disposition", "attachment; filename=\"boosts.csv\"")
                .body(format!("{}", csv).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting boosts.").into())
                .unwrap();
        }
    }
}