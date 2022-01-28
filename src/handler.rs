use crate::{Context, Response};
use hyper::StatusCode;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use voca_rs::*;
use handlebars::Handlebars;
use serde_json::json;



//Constants --------------------------------------------------------------------------------------------------
const WEBROOT_PATH_HTML: &str = "webroot/html";
const WEBROOT_PATH_IMAGE: &str = "webroot/image";
const WEBROOT_PATH_STYLE: &str = "webroot/style";
const WEBROOT_PATH_SCRIPT: &str = "webroot/script";


//Structs and Enums ------------------------------------------------------------------------------------------
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

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/home.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

pub async fn pewmp3(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/pew.mp3").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "audio/mpeg")
        .body(hyper::Body::from(file))
        .unwrap();
}

pub async fn favicon(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/favicon.ico").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "image/x-icon")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Serve a web asset by name from webroot subfolder according to it's requested type
pub async fn asset(ctx: Context) -> Response {
    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("** Context: {:#?}", ctx);
    println!("** Params: {:#?}", _params);

    //Set up the response framework
    let file_path;
    let content_type;
    let file_extension;
    match ctx.path.as_str() {
        "/html" => {
            file_path = WEBROOT_PATH_HTML;
            content_type = "text/html";
            file_extension = "html";
        },
        "/image" => {
            file_path = WEBROOT_PATH_IMAGE;
            content_type = "image/png";
            file_extension = "png";
        },
        "/style" => {
            file_path = WEBROOT_PATH_STYLE;
            content_type = "text/css";
            file_extension = "css";
        },
        "/script" => {
            file_path = WEBROOT_PATH_SCRIPT;
            content_type = "text/javascript";
            file_extension = "js";
        },
        _ => {
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** Invalid asset type requested (ex. /images?name=filename.").into())
                .unwrap();
        },
    };

    //Attempt to serve the file
    if let Some(filename) = _params.get("name") {
        let file_to_serve = format!("{}/{}.{}", file_path, filename, file_extension);
        println!("** Serving file: [{}]", file_to_serve);
        let file = fs::read(file_to_serve.as_str()).expect("Something went wrong reading the file.");
        return hyper::Response::builder()
            .status(StatusCode::OK)
            .header("Content-type", content_type)
            .body(hyper::Body::from(file))
            .unwrap();
    } else {
        return hyper::Response::builder()
            .status(StatusCode::from_u16(500).unwrap())
            .body(format!("** No file specified.").into())
            .unwrap();
    }
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
                },
                Err(_) => {
                    eprintln!("** Error getting boosts: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        },
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
                },
                Err(_) => {
                    eprintln!("** Error getting boosts: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        },
        None => {
            eprintln!("** Error getting boosts: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db_descending(&_ctx.database_file_path, index, boostcount) {
        Ok(boosts) => {
            let json_doc_raw = serde_json::to_string_pretty(&boosts).unwrap();
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
                },
                Err(_) => {
                    eprintln!("** Error getting streams: 'index' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'index' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        },
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
                },
                Err(_) => {
                    eprintln!("** Error getting streams: 'count' param is not a number.\n");
                    return hyper::Response::builder()
                        .status(StatusCode::from_u16(400).unwrap())
                        .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                        .unwrap();
                }
            };
        },
        None => {
            eprintln!("** Error getting streams: 'count' param is not present.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** 'count' is a required parameter and must be an unsigned integer.").into())
                .unwrap();
        }
    };

    //Get the boosts from db for returning
    match dbif::get_streams_from_db_descending(&_ctx.database_file_path, index, boostcount) {
        Ok(streams) => {
            let json_doc_raw = serde_json::to_string_pretty(&streams).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
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


pub async fn api_v1_index(_ctx: Context) -> Response {

    //Get the last known invoice index from the database
    match dbif::get_last_boost_index_from_db(&_ctx.database_file_path) {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            let json_doc_raw = serde_json::to_string_pretty(&index).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .body(format!("{}", json_doc).into())
                .unwrap();
        },
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            return hyper::Response::builder()
                .status(StatusCode::from_u16(500).unwrap())
                .body(format!("** Error getting current db index.").into())
                .unwrap();
        }
    };
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
    let mut last_index = match dbif::get_last_boost_index_from_db(&_ctx.database_file_path) {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            index
        },
        Err(_) => 0
    };
    if last_index > boostcount && !old {
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
    match dbif::get_boosts_from_db(&_ctx.database_file_path, index, boostcount, old) {
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