use crate::{Context, Request, Body, Response};
use crate::lightning;
use crate::podcastindex;
use crate::cookies::CookiesExt;
use cookie::Cookie;
use hyper::{Method, StatusCode};
use hyper::header;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::str;
use voca_rs::*;
use handlebars::Handlebars;
use serde_json::json;
use chrono::{Duration, NaiveDateTime, Utc};
use dbif::BoostRecord;

use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, Algorithm, Header, DecodingKey, EncodingKey, Validation};

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

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
   // sub: String,
   iat: usize,
   exp: usize,
}

//Helper functions
async fn get_post_params(req: Request<Body>) -> HashMap<String, String> {
    let full_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let body_str = str::from_utf8(&full_body).unwrap();
    let body_params = url::form_urlencoded::parse(body_str.as_bytes());

    return body_params
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
}

fn client_error_response(message: String) -> Response {
    text_response(message, StatusCode::BAD_REQUEST)
}

fn server_error_response(message: String) -> Response {
    text_response(message, StatusCode::SERVICE_UNAVAILABLE)
}

fn text_response(message: String, code: StatusCode) -> Response {
    return hyper::Response::builder()
        .status(code)
        .body(message.into())
        .unwrap();
}

fn json_response<T: serde::Serialize>(value: T) -> Response {
    let json_doc_raw = serde_json::to_string_pretty(&value).unwrap();
    let json_doc: String = strip::strip_tags(&json_doc_raw);

    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json")
        .body(format!("{}", json_doc).into())
        .unwrap();
}

fn options_response(options: String) -> Response {
    return hyper::Response::builder()
        .status(StatusCode::NO_CONTENT)
        .header("Access-Control-Allow-Methods", options)
        .body(format!("").into())
        .unwrap();
}

pub fn redirect(url: &str) -> Response {
    hyper::Response::builder()
        .status(StatusCode::FOUND)
        .header("Location", url)
        .body("".into())
        .unwrap()
}

pub fn verify_jwt_cookie(req: &Request<Body>, secret: &String) -> bool {
    if secret.is_empty() {
        return false;
    }

    let cookies = req.cookies();

    if let Some(token) = cookies.get("HELIPAD_JWT").map(Cookie::value) {
        let message = decode::<JwtClaims>(&token, &DecodingKey::from_secret(secret.as_ref()), &Validation::new(Algorithm::HS256));

        if let Ok(token) = message {
            let timestamp = Utc::now().timestamp() as usize;

            if token.claims.exp > timestamp {
                return true;
            }
        }
    }

    false
}

pub fn set_jwt_cookie(resp: &mut Response, secret: &String) {
    let iat = Utc::now().timestamp();
    let exp = Utc::now()
        .checked_add_signed(Duration::hours(1))
        .expect("invalid timestamp")
        .timestamp();

    let my_claims = JwtClaims {
        iat: iat as usize,
        exp: exp as usize,
    };

    let token = encode(&Header::default(), &my_claims, &EncodingKey::from_secret(secret.as_ref())).unwrap();

    // Build a session cookie.
    let cookie = Cookie::build(("HELIPAD_JWT", token))
        .path("/")
        .secure(false) // Do not require HTTPS.
        .http_only(true)
        .same_site(cookie::SameSite::Lax)
        .build();

    // Set the changed cookies
    resp.headers_mut().insert(
        header::SET_COOKIE,
        header::HeaderValue::from_str(&cookie.to_string()).unwrap()
    );

}

pub fn login_required(ctx: &Context) -> Option<Response> {
    if ctx.helipad_config.password.is_empty() {
        return None;
    }

    let path = ctx.req.uri().path();

    if path == "/login" || path.starts_with("/script") || path.starts_with("/style") {
        return None;
    }

    if verify_jwt_cookie(&ctx.req, &ctx.helipad_config.secret) {
        return None;
    }

    let ctype = match ctx.req.headers().get(header::CONTENT_TYPE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => "",
    };

    if ctype.starts_with("application/json") {
        return Some(text_response("Access forbidden".into(), StatusCode::FORBIDDEN));
    }

    Some(redirect("/login"))
}

//Route handlers ---------------------------------------------------------------------------------------------

//Login html
pub async fn login(ctx: Context) -> Response {
    if ctx.helipad_config.password.is_empty() {
        return redirect("/"); // no password required
    }

    let mut message = "";

    if ctx.req.method() == Method::POST {
        let post_vars = get_post_params(ctx.req).await;

        if let Some(password) = post_vars.get("password") {
            if ctx.helipad_config.password == *password {
                let mut resp = redirect("/");
                set_jwt_cookie(&mut resp, &ctx.helipad_config.secret);
                return resp;
            }
            else {
                message = "Bad password";
            }
        }
        else {
            message = "No password provided";
        }
    }

    let params = json!({
        "version": ctx.state.version,
        "message": message,
    });

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/login.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &params).expect("Something went wrong rendering the file");

    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Homepage html
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

//Streams html
pub async fn streams(ctx: Context) -> Response {

    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/streams.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Sent html
pub async fn sent(ctx: Context) -> Response {
    let reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/html/sent.html").expect("Something went wrong reading the file.");
    let doc_rendered = reg.render_template(&doc, &json!({"version": ctx.state.version})).expect("Something went wrong rendering the file");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc_rendered).into())
        .unwrap();
}

//Pew-pew audio
pub async fn pewmp3(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/pew.mp3").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "audio/mpeg")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Favicon icon
pub async fn favicon(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/favicon.ico").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "image/x-icon")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Apps definitions file
pub async fn apps_json(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/apps.json").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Numerology definitions file
pub async fn numerology_json(_ctx: Context) -> Response {
    let file = fs::read("webroot/extra/numerology.json").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(hyper::Body::from(file))
        .unwrap();
}

//Serve a web asset by name from webroot subfolder according to it's requested type
pub async fn asset(ctx: Context) -> Response {
    //Get query parameters
    let _params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("** Request: {:#?}", ctx.req);
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
        }
        "/image" => {
            file_path = WEBROOT_PATH_IMAGE;
            content_type = "image/png";
            file_extension = "png";
        }
        "/style" => {
            file_path = WEBROOT_PATH_STYLE;
            content_type = "text/css";
            file_extension = "css";
        }
        "/script" => {
            file_path = WEBROOT_PATH_SCRIPT;
            content_type = "text/javascript";
            file_extension = "js";
        }
        _ => {
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("** Invalid asset type requested (ex. /images?name=filename.").into())
                .unwrap();
        }
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
    match dbif::get_wallet_balance_from_db(&_ctx.helipad_config.database_file_path) {
        Ok(balance) => {
            let json_doc = serde_json::to_string_pretty(&balance).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", "application/json")
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
    match dbif::get_boosts_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(boosts) => {
            let json_doc = serde_json::to_string_pretty(&boosts).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", "application/json")
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
    match dbif::get_streams_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(streams) => {
            let json_doc_raw = serde_json::to_string_pretty(&streams).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", "application/json")
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
    match dbif::get_last_boost_index_from_db(&_ctx.helipad_config.database_file_path) {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            let json_doc_raw = serde_json::to_string_pretty(&index).unwrap();
            let json_doc: String = strip::strip_tags(&json_doc_raw);

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .header("Access-Control-Allow-Origin", "*")
                .header("Content-Type", "application/json")
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

//API - get the current payment index number
pub async fn api_v1_sent_index_options(_ctx: Context) -> Response {
    options_response("GET, OPTIONS".into())
}

pub async fn api_v1_sent_index(_ctx: Context) -> Response {
    //Get the last known payment index from the database
    match dbif::get_last_payment_index_from_db(&_ctx.helipad_config.database_file_path) {
        Ok(index) => {
            println!("** get_last_payment_index_from_db() -> [{}]", index);
            json_response(index)
        }
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            server_error_response("** Error getting current db index.".into())
        }
    }
}

//API - serve sent as JSON either in ascending or descending order
pub async fn api_v1_sent_options(_ctx: Context) -> Response {
    options_response("GET, OPTIONS".into())
}

pub async fn api_v1_sent(_ctx: Context) -> Response {
    //Get query parameters
    let params: HashMap<String, String> = _ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    //Parameter - index (unsigned int)
    let index = match params.get("index") {
        Some(supplied_index) => {
            match supplied_index.parse::<u64>() {
                Ok(index) => {
                    println!("** Supplied index from call: [{}]", index);
                    index
                }
                Err(_) => {
                    eprintln!("** Error getting sent boosts: 'index' param is not a number.\n");
                    return client_error_response("** 'index' is a required parameter and must be an unsigned integer.".into());
                }
            }
        }
        None => {
            eprintln!("** Error getting sent boosts: 'index' param is not present.\n");
            return client_error_response("** 'index' is a required parameter and must be an unsigned integer.".into())
        }
    };

    //Parameter - boostcount (unsigned int)
    let boostcount = match params.get("count") {
        Some(bcount) => {
            match bcount.parse::<u64>() {
                Ok(boostcount) => {
                    println!("** Supplied sent boost count from call: [{}]", boostcount);
                    boostcount
                }
                Err(_) => {
                    eprintln!("** Error getting sent boosts: 'count' param is not a number.\n");
                    return client_error_response("** 'count' is a required parameter and must be an unsigned integer.".into())
                }
            }
        }
        None => {
            eprintln!("** Error getting sent boosts: 'count' param is not present.\n");
            return client_error_response("** 'count' is a required parameter and must be an unsigned integer.".into())
        }
    };

    //Parameter - old (bool)
    let old = match params.get("old") {
        Some(old_val) => match old_val.parse::<bool>() {
            Ok(val) => val,
            Err(_) => false,
        },
        None => false,
    };

    //Get sent boosts from db for returning
    match dbif::get_payments_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(sent_boosts) => {
            json_response(sent_boosts)
        }
        Err(e) => {
            eprintln!("** Error getting sent boosts: {}.\n", e);
            server_error_response("** Error getting sent boosts.".into())
        }
    }
}

pub async fn api_v1_reply_options(_ctx: Context) -> Response {
    options_response("POST, OPTIONS".to_string())
}

pub async fn api_v1_reply(_ctx: Context) -> Response {
    let post_vars = get_post_params(_ctx.req).await;

    //Parameter - index (unsigned int)
    let index = match post_vars.get("index") {
        Some(index) => match index.parse::<u64>() {
            Ok(index) => index,
            Err(_) => {
                eprintln!("** Error parsing reply params: 'index' param is not a number.\n");
                return client_error_response("** 'index' is a required parameter and must be an unsigned integer.".into());
            }
        },
        None => {
            return client_error_response("** No index specified.".to_string());
        },
    };

    //Parameter - sats (unsigned int)
    let sats = match post_vars.get("sats") {
        Some(sats) => match sats.parse::<u64>() {
            Ok(sats) => sats,
            Err(_) => {
                eprintln!("** Error parsing reply params: 'sats' param is not a number.\n");
                return client_error_response("** 'sats' is a required parameter and must be an unsigned integer.".into());
            }
        },
        None => {
            return client_error_response("** No sats specified.".to_string());
        },
    };

    let sender = match post_vars.get("sender") {
        Some(name) => name,
        None => "Anonymous"
    };

    let message = match post_vars.get("message") {
        Some(msg) => msg,
        None => ""
    };

    let boosts = match dbif::get_boosts_from_db(&_ctx.helipad_config.database_file_path, index, 1, true, true) {
        Ok(items) => items,
        Err(_) => {
            return server_error_response("** Error finding boost index.".to_string());
        }
    };

    if boosts.is_empty() {
        return server_error_response("** Unknown boost index.".to_string());
    }

    let boost = &boosts[0];
    let tlv = boost.parse_tlv().unwrap();

    let pub_key = tlv["reply_address"].as_str().unwrap_or_default().to_string();
    let custom_key = tlv["reply_custom_key"].as_u64();
    let custom_value = match tlv["reply_custom_value"].as_str() {
        Some(rcv) => Some(rcv.to_string()),
        None => None
    };

    if pub_key == "" {
        return client_error_response("** No reply_address found in boost".to_string());
    }

    if custom_key.is_some() && custom_value.is_none() {
        return client_error_response("** No reply_custom_value found in boost".to_string());
    }

    let reply_tlv = json!({
        "app_name": "Helipad",
        "app_version": _ctx.state.version,
        "podcast": tlv["podcast"].as_str().unwrap_or_default(),
        "episode": tlv["episode"].as_str().unwrap_or_default(),
        "name": tlv["sender_name"].as_str().unwrap_or_default(),
        "sender_name": sender,
        "message": message,
        "action": "boost",
        "value_msat": sats * 1000,
        "value_msat_total": sats * 1000,
    });

    let helipad_config = _ctx.helipad_config.clone();
    let lightning = match lightning::connect_to_lnd(helipad_config.node_address, helipad_config.cert_path, helipad_config.macaroon_path).await {
        Some(lndconn) => lndconn,
        None => {
            return server_error_response("** Error connecting to LND.".to_string())
        }
    };

    let payment = match lightning::send_boost(lightning, pub_key, custom_key, custom_value, sats, reply_tlv.clone()).await {
        Ok(payment) => payment,
        Err(e) => {
            eprintln!("** Error sending boost: {}", e);
            return server_error_response(format!("** Error sending boost: {}", e))
        }
    };

    let mut cache = podcastindex::GuidCache::new(1);

    let mut boost = match lightning::parse_boost_from_payment(payment, &mut cache).await {
        Some(boost) => boost,
        None => {
            eprintln!("** Error parsing sent boost");
            return server_error_response("** Error parsing sent boost".into())
        }
    };

    if let Some(pay_info) = boost.payment_info {
        boost.payment_info = Some(dbif::PaymentRecord {
            reply_to_idx: Some(index),
            ..pay_info
        });
    }

    //Give some output
    println!("Sent Boost: {:#?}", boost);

    //Store in the database
    match dbif::add_payment_to_db(&_ctx.helipad_config.database_file_path, &boost) {
        Ok(_) => println!("New sent boost added."),
        Err(e) => eprintln!("Error adding sent boost: {:#?}", e)
    }

    json_response(json!({
        "success": true,
        "data": boost,
    }))
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
    match dbif::get_boosts_from_db(&_ctx.helipad_config.database_file_path, index, boostcount, old, false) {
        Ok(boosts) => {
            let mut csv = String::new();

            //CSV column name header
            csv.push_str(format!("count,index,time,value_msat,value_sat,value_msat_total,value_sat_total,action,sender,app,message,podcast,episode,remote_podcast,remote_episode\n").as_str());

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
                        "{},{},\"{}\",{},{},{},{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
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
                        BoostRecord::escape_for_csv(boost.episode),
                        BoostRecord::escape_for_csv(boost.remote_podcast.unwrap_or("".to_string())),
                        BoostRecord::escape_for_csv(boost.remote_episode.unwrap_or("".to_string()))
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