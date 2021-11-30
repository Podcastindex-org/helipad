use crate::{Context, Response};
use hyper::StatusCode;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use voca_rs::*;


//Structs ----------------------------------------------------------------------------------------------------
#[derive(Debug)]
struct HydraError(String);
impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}
impl Error for HydraError {}


//Functions --------------------------------------------------------------------------------------------------
pub async fn home(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //println!("** Params: {:#?}", _params);

    let doc = fs::read_to_string("home.html").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc).into())
        .unwrap();
}

pub async fn pewmp3(_ctx: Context) -> Response {
    let file = fs::read("pew.mp3").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "audio/mpeg")
        .body(hyper::Body::from(file))
        .unwrap();
}

pub async fn favicon(_ctx: Context) -> Response {
    let file = fs::read("favicon.ico").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "image/x-icon")
        .body(hyper::Body::from(file))
        .unwrap();
}

pub async fn homejs(_ctx: Context) -> Response {
    let doc = fs::read_to_string("home.js").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc).into())
        .unwrap();
}

pub async fn utilsjs(_ctx: Context) -> Response {
    let doc = fs::read_to_string("utils.js").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc).into())
        .unwrap();
}

pub async fn boosts(_ctx: Context) -> Response {
    let default_boostcount: u64 = 50;

    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //println!("Params: {:#?}", params);
    println!("");

    //Get the count parameter if one was given and convert to an integer
    let boostcount: u64;
    match params.get("count") {
        Some(bcount) => {
            boostcount = match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied boostcount from call: [{}]", boostcount);
                    boostcount
                },
                Err(_) => default_boostcount
            };
        },
        None => {
            println!("** No boostcount given.  Using: [{}]", default_boostcount);
            boostcount = default_boostcount;
        }
    };

    //Was the "old" flag used?
    let mut old = false;
    match params.get("old") {
        Some(_) => old = true,
        None => { }
    };


    //Get the last known invoice index from the database
    let mut last_index = match dbif::get_last_boost_index_from_db() {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            index
        },
        Err(_) => 0
    };
    if last_index > boostcount {
        last_index -= boostcount;
    }

    //Get the index url parameter if one was given and convert to an integer
    //If one wasn't given, just use what we calculated above
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                },
                Err(_) => last_index
            };
        },
        None => {
            println!("** No index given.  Using: [{}]", last_index);
            index = last_index;
        }
    };


    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(index, boostcount, old) {
        Ok(boosts) => {
            let json_doc_raw = serde_json::to_string(&boosts).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
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


