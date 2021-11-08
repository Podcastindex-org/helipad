use crate::{Context, Response};
use hyper::StatusCode;
use std::collections::HashMap;
//use rusqlite::{params, Connection};
use std::error::Error;
use std::fmt;
//use std::time::{SystemTime};
//use percent_encoding::percent_decode;
use std::fs;
//use serde::{Deserialize, Serialize};
//use serde_json::Result;

//Globals ----------------------------------------------------------------------------------------------------
// const SQLITE_FILE_AUTH: &str = "auth.db";
// const SQLITE_FILE_QUEUE: &str = "queue.db";
// const SQLITE_FILE_COMMENTS: &str = "comments.db";


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

    //Get the index url parameter if one was given and convert to an integer
    let mut index: u64 = 0;
    let supplied_index = match params.get("index") {
        Some(supplied_index) => {
            index = match supplied_index.parse::<u64>() {
                Ok(index) => index,
                Err(_) => 0
            };
        },
        None => {
            index = match dbif::get_last_boost_index_from_db() {
                Ok(index) => index,
                Err(_) => 0
            };
            if index > 20 {
                index -= 20;
            }
        }
    };


    //If zero was given or no value supplied

    match dbif::get_boosts_from_db(index, 20) {
        Ok(boosts) => {
            let json_doc = serde_json::to_string(&boosts).unwrap();

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


