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
    let params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("Params: {:#?}", params);

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

    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("Params: {:#?}", params);

    //Get the last known invoice index from the database
    let mut last_index = match dbif::get_last_boost_index_from_db() {
        Ok(index) => index,
        Err(_) => 0
    };
    if last_index > 20 {
        last_index -= 20;
    }

    //Get the index url parameter if one was given and convert to an integer
    //If one wasn't given, just use what we calculated above
    let index: u64;
    match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => index,
                Err(_) => last_index
            };
        },
        None => {
            index = last_index;
        }
    };


    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(index, 20) {
        Ok(boosts) => {
            let json_doc_raw = serde_json::to_string(&boosts).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .body(format!("{}", json_doc).into())
                .unwrap();
        }
        Err(e) => {
            eprintln!("Error getting boosts: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("Error getting boosts.").into())
                .unwrap();
        }
    }

}


