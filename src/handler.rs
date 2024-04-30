// use crate::{Context, Request, Body, Response};
use axum::{
    body::Body,
    extract::{Form, Path, Query, Request, State},
    http::{header, StatusCode, Uri},
    middleware::Next,
    response::{Html, Json, Redirect, IntoResponse, Response},
};

use axum_extra::{
    extract::cookie::{CookieJar, Cookie},
};

use chrono::{DateTime, TimeDelta, Utc};
use crate::{AppState, lightning, podcastindex};
use dbif::{BoostRecord, WebhookRecord};
use handlebars::{Handlebars, JsonRender};
use jsonwebtoken::{decode, encode, Algorithm, Header, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, str};
use url::Url;

//Constants --------------------------------------------------------------------------------------------------
const WEBROOT_PATH_HTML: &str = "webroot/html";
const WEBROOT_PATH_IMAGE: &str = "webroot/image";
const WEBROOT_PATH_STYLE: &str = "webroot/style";
const WEBROOT_PATH_SCRIPT: &str = "webroot/script";


//Structs and Enums ------------------------------------------------------------------------------------------
#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
   // sub: String,
   iat: usize,
   exp: usize,
}

pub fn verify_jwt_cookie(jar: &CookieJar, secret: &String) -> bool {
    if secret.is_empty() {
        return false; // no secret
    }

    let jwt = match jar.get("HELIPAD_JWT") {
        Some(jwt) => jwt.value_trimmed(),
        None => {
            eprintln!("No HELIPAD_JWT cookie found");
            return false; // no cookie
        }
    };

    let token = match decode::<JwtClaims>(&jwt, &DecodingKey::from_secret(secret.as_ref()), &Validation::new(Algorithm::HS256)) {
        Ok(token) => token,
        Err(_) => {
            eprintln!("Unable to decode HELIPAD_JWT cookie");
            return false;
        }
    };

    let timestamp = Utc::now().timestamp() as usize;

    if token.claims.exp <= timestamp {
        eprintln!("Expired HELIPAD_JWT cookie");
        return false; // expired
    }

    true
}

pub fn new_jwt_cookie(jar: CookieJar, secret: String) -> CookieJar {
    let iat = Utc::now().timestamp();
    let exp = Utc::now()
        .checked_add_signed(TimeDelta::try_hours(1).unwrap())
        .expect("invalid timestamp")
        .timestamp();

    let my_claims = JwtClaims {
        iat: iat as usize,
        exp: exp as usize,
    };

    let jwt = encode(&Header::default(), &my_claims, &EncodingKey::from_secret(secret.as_ref())).unwrap();

    let cookie = Cookie::build(("HELIPAD_JWT", jwt))
        .path("/")
        .secure(false) // Do not require HTTPS.
        .http_only(true)
        .same_site(cookie::SameSite::Lax)
        .build();

    // Add cookie to jar and return
    jar.add(cookie)
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    jar: CookieJar,
    req: Request,
    next: Next,
) -> Response {

    if state.helipad_config.password.is_empty() {
        return next.run(req).await; // no password required
    }

    let path = req.uri().path();

    if path == "/login" || path.starts_with("/script") || path.starts_with("/style") {
        return next.run(req).await; // no password required for certain paths
    }

    if verify_jwt_cookie(&jar, &state.helipad_config.secret) {
        // valid jwt: refresh and add to response
        let resp = next.run(req).await;
        let cookie = new_jwt_cookie(jar, state.helipad_config.secret);
        return (cookie, resp).into_response();
    }

    let ctype = match req.headers().get(header::CONTENT_TYPE) {
        Some(val) => val.to_str().unwrap_or(""),
        None => "",
    };

    // login required
    if ctype.starts_with("application/json") {
        return (StatusCode::FORBIDDEN, "Access forbidden").into_response(); // json response
    }

    Redirect::to("/login").into_response() // redirect to login
}

//Login html
#[derive(Debug, Serialize, Deserialize)]
pub struct LoginForm {
    password: String,
}

pub async fn login(State(state): State<AppState>) -> Response {
    if state.helipad_config.password.is_empty() {
        return Redirect::to("/").into_response(); // no password required
    }

    HtmlTemplate("webroot/html/login.html", json!({"message": ""})).into_response()
}

pub async fn handle_login(
    State(state): State<AppState>,
    jar: CookieJar,
    Form(post_vars): Form<LoginForm>,
) -> Response {

    if state.helipad_config.password == *post_vars.password {
        // valid password: set cookie and redirect
        let cookie = new_jwt_cookie(jar, state.helipad_config.secret);
        let resp = Redirect::to("/");
        return (cookie, resp).into_response();
    }

    HtmlTemplate("webroot/html/login.html", json!({
        "version": state.version,
        "message": "Bad password",
    })).into_response()
}


struct HtmlTemplate<'a, T>(&'a str, T);

impl<T> IntoResponse for HtmlTemplate<'_, T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let reg = Handlebars::new();

        let doc = match fs::read_to_string(self.0) {
            Ok(doc) => doc,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Unable to open template file").into_response();
            }
        };

        let doc_rendered = match reg.render_template(&doc, &self.1) {
            Ok(rendered) => rendered,
            Err(_) => {
                return (StatusCode::BAD_REQUEST, "Unable to render template").into_response();
            }
        };

        Html(doc_rendered).into_response()
    }
}

//Homepage html
pub async fn home(State(state): State<AppState>) -> Response {
    HtmlTemplate("webroot/html/home.html", &json!({"version": state.version})).into_response()
}

//Streams html
pub async fn streams(State(state): State<AppState>) -> Response {
    HtmlTemplate("webroot/html/streams.html", &json!({"version": state.version})).into_response()
}

//Sent html
pub async fn sent(State(state): State<AppState>) -> Response {
    HtmlTemplate("webroot/html/sent.html", &json!({"version": state.version})).into_response()
}

//Streams html
pub async fn settings(State(state): State<AppState>) -> Response {
    HtmlTemplate("webroot/html/settings.html", &json!({"version": state.version})).into_response()
}

//Pew-pew audio
pub async fn pewmp3() -> Response {
    let file = fs::read("webroot/extra/pew.mp3").expect("Unable to read file");
    ([(header::CONTENT_TYPE, "audio/mpeg")], file).into_response()
}

//Favicon icon
pub async fn favicon() -> Response {
    let file = fs::read("webroot/extra/favicon.ico").expect("Unable to read file");
    ([(header::CONTENT_TYPE, "image/png")], file).into_response()
}

//Apps definitions file
pub async fn apps_json() -> Response {
    let file = fs::read("webroot/extra/apps.json").expect("Unable to read file");
    ([(header::CONTENT_TYPE, "application/json")], file).into_response()
}

//Numerology definitions file
pub async fn numerology_json() -> Response {
    let file = fs::read("webroot/extra/numerology.json").expect("Unable to read file");
    ([(header::CONTENT_TYPE, "application/json")], file).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetParams {
    name: String,
}

//Serve a web asset by name from webroot subfolder according to it's requested type
pub async fn asset(
    Query(params): Query<AssetParams>,
    uri: Uri,
) -> Response {
    println!("** Uri: {:#?}", uri);
    println!("** Params: {:#?}", params);

    //Set up the response framework
    let file_path;
    let content_type;
    let file_extension;

    match uri.path() {
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
            return (StatusCode::BAD_REQUEST, "** Invalid asset type requested (ex. /images?name=filename.").into_response();
        }
    };

    //Attempt to serve the file
    let file_to_serve = format!("{}/{}.{}", file_path, params.name, file_extension);
    println!("** Serving file: [{}]", file_to_serve);
    let file = fs::read(file_to_serve.as_str()).expect("Something went wrong reading the file.");
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, content_type)],
        Body::from(file)
    ).into_response()
}

//API - give back node info
pub async fn api_v1_node_info_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

pub async fn api_v1_node_info(State(state): State<AppState>) -> Response {
    match dbif::get_node_info_from_db(&state.helipad_config.database_file_path) {
        Ok(info) => {
            Json(info).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting node info: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting node info.").into_response()
        }
    }
}

//API - give back the node balance
pub async fn api_v1_balance_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

pub async fn api_v1_balance(State(state): State<AppState>) -> Response {

    //Get the boosts from db for returning
    match dbif::get_wallet_balance_from_db(&state.helipad_config.database_file_path) {
        Ok(balance) => {
            Json(balance).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting balance: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting balance.").into_response()
        }
    }
}

//API - serve boosts as JSON either in ascending or descending order
pub async fn api_v1_boosts_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoostParams {
    index: u64,
    count: u64,
    old: Option<bool>,
}

impl Default for BoostParams {
    fn default() -> Self {
        Self { index: 0, count: 0, old: Some(false) }
    }
}

pub async fn api_v1_boosts(
    Query(params): Query<BoostParams>,
    State(state): State<AppState>,
) -> Response {

    let index = params.index;
    let boostcount = params.count;

    //Was the "old" flag used?
    let mut old = false;
    match params.old {
        Some(_) => old = true,
        None => {}
    };

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied boost count from call: [{}]", boostcount);

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(&state.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(boosts) => {
            Json(boosts).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting boosts.").into_response()
        }
    }
}

//API - serve streams as JSON either in ascending or descending order
pub async fn api_v1_streams_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

pub async fn api_v1_streams(
    params: Option<Query<BoostParams>>,
    State(state): State<AppState>
) -> Response {
    let Query(params) = params.unwrap_or_default();

    let index = params.index;
    let boostcount = params.count;

    //Was the "old" flag used?
    let mut old = false;
    match params.old {
        Some(_) => old = true,
        None => {}
    };

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied stream count from call: [{}]", boostcount);

    //Get the boosts from db for returning
    match dbif::get_streams_from_db(&state.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(streams) => {
            Json(streams).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting streams: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting streams.").into_response()
        }
    }
}

//API - get the current invoice index number
pub async fn api_v1_index_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

pub async fn api_v1_index(State(state): State<AppState>) -> Response {

    //Get the last known invoice index from the database
    match dbif::get_last_boost_index_from_db(&state.helipad_config.database_file_path) {
        Ok(index) => {
            println!("** get_last_boost_index_from_db() -> [{}]", index);
            Json(index).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting current db index.").into_response()
        }
    }
}

//API - get the current payment index number
pub async fn api_v1_sent_index_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

pub async fn api_v1_sent_index(State(state): State<AppState>) -> Response {
    //Get the last known payment index from the database
    match dbif::get_last_payment_index_from_db(&state.helipad_config.database_file_path) {
        Ok(index) => {
            println!("** get_last_payment_index_from_db() -> [{}]", index);
            Json(index).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting current db index: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting current sent index.").into_response()
        }
    }
}

//API - serve sent as JSON either in ascending or descending order
pub async fn api_v1_sent_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

pub async fn api_v1_sent(
    params: Option<Query<BoostParams>>,
    State(state): State<AppState>
) -> Response {
    let Query(params) = params.unwrap_or_default();

    let index = params.index;
    let boostcount = params.count;

    //Was the "old" flag used?
    let mut old = false;
    match params.old {
        Some(_) => old = true,
        None => {}
    };

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied sent boost count from call: [{}]", boostcount);

    //Get sent boosts from db for returning
    match dbif::get_payments_from_db(&state.helipad_config.database_file_path, index, boostcount, old, true) {
        Ok(sent_boosts) => {
            Json(sent_boosts).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting sent boosts: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting sent boosts.").into_response()
        }
    }
}

pub async fn api_v1_reply_options() -> impl IntoResponse {
    (
        StatusCode::NO_CONTENT,
        [(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS")],
        ""
    )
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplyForm {
    index: u64,
    sats: u64,
    sender: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplyResponse {
    success: bool,
    data: BoostRecord,
}

pub async fn api_v1_reply(
    State(state): State<AppState>,
    Form(params): Form<ReplyForm>,
) -> Response {
    let index = params.index;
    let sats = params.sats;
    let sender = params.sender.unwrap_or("Anonymous".into());
    let message = params.message.unwrap_or("".into());

    let boosts = match dbif::get_boosts_from_db(&state.helipad_config.database_file_path, index, 1, true, true) {
        Ok(items) => items,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error finding boost index.").into_response();
        }
    };

    if boosts.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, "** Unknown boost index.").into_response();
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
        return (StatusCode::BAD_REQUEST, "** No reply_address found in boost").into_response();
    }

    if custom_key.is_some() && custom_value.is_none() {
        return (StatusCode::BAD_REQUEST, "** No reply_custom_value found in boost").into_response();
    }

    let reply_tlv = json!({
        "app_name": "Helipad",
        "app_version": state.version,
        "podcast": tlv["podcast"].as_str().unwrap_or_default(),
        "episode": tlv["episode"].as_str().unwrap_or_default(),
        "name": tlv["sender_name"].as_str().unwrap_or_default(),
        "sender_name": sender,
        "message": message,
        "action": "boost",
        "value_msat": sats * 1000,
        "value_msat_total": sats * 1000,
    });

    let helipad_config = state.helipad_config.clone();
    let lightning = match lightning::connect_to_lnd(helipad_config.node_address, helipad_config.cert_path, helipad_config.macaroon_path).await {
        Some(lndconn) => lndconn,
        None => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error connecting to LND.").into_response();
        }
    };

    let payment = match lightning::send_boost(lightning, pub_key, custom_key, custom_value, sats, reply_tlv.clone()).await {
        Ok(payment) => payment,
        Err(e) => {
            eprintln!("** Error sending boost: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("** Error sending boost: {}", e)).into_response();
        }
    };

    let mut cache = podcastindex::GuidCache::new(1);

    let mut boost = match lightning::parse_boost_from_payment(payment, &mut cache).await {
        Some(boost) => boost,
        None => {
            eprintln!("** Error parsing sent boost");
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error parsing sent boost").into_response();
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
    match dbif::add_payment_to_db(&state.helipad_config.database_file_path, &boost) {
        Ok(_) => println!("New sent boost added."),
        Err(e) => eprintln!("Error adding sent boost: {:#?}", e)
    }

    Json(ReplyResponse {
        success: true,
        data: boost,
    }).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkRepliedForm {
    index: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarkRepliedResponse {
    success: bool,
}

pub async fn api_v1_mark_replied(
    State(state): State<AppState>,
    Form(params): Form<MarkRepliedForm>,
) -> Response {

    //Parameter - index (unsigned int)
    let index = params.index;

    let result = dbif::mark_boost_as_replied(&state.helipad_config.database_file_path, index);

    if let Err(e) = result {
        eprintln!("** Error marking boost as replied: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("** Error marking boost as replied: {}", e)).into_response();
    }

    Json(MarkRepliedResponse {
        success: true,
    }).into_response()
}

async fn webhook_list_response(db_filepath: &String) -> Response {
    let webhooks = match dbif::get_webhooks_from_db(&db_filepath, None) {
        Ok(wh) => wh,
        Err(e) => {
            eprintln!("** Error getting webhooks: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("** Error getting webhooks.")).into_response();
        }
    };

    println!("** get_webhooks_from_db()");

    let mut reg = Handlebars::new();
    let doc = fs::read_to_string("webroot/template/webhook-list.hbs").expect("Something went wrong reading the file.");

    reg.register_helper("timestamp", Box::new(|h: &handlebars::Helper, _: &handlebars::Handlebars, _: &handlebars::Context, _: &mut handlebars::RenderContext, out: &mut dyn handlebars::Output| -> handlebars::HelperResult {
        let param = h.param(0).unwrap();
        let timestamp = param.value().render().parse::<i64>().unwrap();

        if let Some(ts) = DateTime::from_timestamp(timestamp, 0) {
            let _ = out.write(&ts.to_rfc3339());
        }

        Ok(())
    }));

    let doc_rendered = reg.render_template(&doc, &json!({"webhooks": webhooks})).expect("Something went wrong rendering the file");

    Html(doc_rendered).into_response()
}

pub async fn webhook_settings_list(State(state): State<AppState>) -> Response {
    webhook_list_response(&state.helipad_config.database_file_path).await
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEditParams {
    idx: String,
}

impl Default for WebhookEditParams {
    fn default() -> Self {
        Self { idx: String::from("") }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookEditResponse {
    webhook: Option<WebhookRecord>,
}

pub async fn webhook_settings_load(
    Path(idx): Path<String>,
    State(state): State<AppState>
) -> Response {
    // let Query(params) = params.unwrap_or_default();

    let index = match idx.as_str() {
        "add" => 0,
        idx => idx.parse().unwrap(),
    };

    let mut result = WebhookEditResponse{
        webhook: None,
    };

    if index > 0 {
        let webhook = match dbif::load_webhook_from_db(&state.helipad_config.database_file_path, index) {
            Ok(wh) => wh,
            Err(e) => {
                eprintln!("** Error loading webhook: {}.\n", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("** Error loading webhook.")).into_response();
            }
        };

        result = WebhookEditResponse{
            webhook: Some(webhook),
        };
    }

    println!("** load_webhook_from_db({})", index);

    HtmlTemplate("webroot/template/webhook-edit.hbs", &result).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookSaveParams {
    idx: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookSaveForm {
    url: String,
    token: String,
    on_boost: Option<bool>,
    on_stream: Option<bool>,
    on_sent: Option<bool>,
    enabled: Option<bool>,
}

pub async fn webhook_settings_save(
    State(state): State<AppState>,
    Path(idx): Path<String>,
    Form(form): Form<WebhookSaveForm>,
) -> Response {
    let db_filepath = state.helipad_config.database_file_path;

    let index = match idx.as_str() {
        "add" => 0,
        idx => idx.parse().unwrap(),
    };

    if let Err(e) = Url::parse(form.url.as_str()) {
        return (StatusCode::BAD_REQUEST, format!("** bad value for url: {}", e)).into_response();
    }

    let webhook = WebhookRecord {
        index: index,
        url: form.url,
        token: form.token,
        on_boost: form.on_boost.unwrap_or(false),
        on_stream: form.on_stream.unwrap_or(false),
        on_sent: form.on_sent.unwrap_or(false),
        enabled: form.enabled.unwrap_or(false),
        request_successful: None,
        request_timestamp: None,
    };

    let idx = match dbif::save_webhook_to_db(&db_filepath, &webhook) {
        Ok(idx) => idx,
        Err(e) => {
            eprintln!("** Error saving webhook: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error saving webhook.").into_response();
        }
    };

    println!("** save_webhook_from_db({})", idx);

    webhook_list_response(&db_filepath).await
}

pub async fn webhook_settings_delete(
    State(state): State<AppState>,
    Path(idx): Path<String>
) -> impl IntoResponse {

    let index = idx.parse().unwrap();

    if let Err(e) = dbif::delete_webhook_from_db(&state.helipad_config.database_file_path, index) {
        eprintln!("** Error deleting webhook: {}.\n", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "** Error deleting webhook.");
    }

    println!("** delete_webhook_from_db({})", index);

    (StatusCode::OK, "")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CsvParams {
    list: Option<String>,
    index: u64,
    count: u64,
    old: Option<bool>,
    end: Option<u64>,
}

impl Default for CsvParams {
    fn default() -> Self {
        Self { list: Some(String::from("boosts")), index: 0, count: 0, old: None, end: None }
    }
}

//CSV export - max is 200 for now so the csv content can be built in memory
pub async fn csv_export_boosts(
    State(state): State<AppState>,
    Query(params): Query<CsvParams>,
) -> Response {
    //Parameter - list (String)
    let list = match params.list {
        Some(name) => name,
        None => "boosts".to_string(),
    };

    //Parameter - index (unsigned int)
    let index = params.index;

    //Parameter - boostcount (unsigned int)
    let boostcount = params.count;

    //Was the "old" flag used?
    let old = match params.old {
        Some(_) => true,
        None => false,
    };

    //Was a stop index given?
    let endex = match params.end {
        Some(endexnum) => endexnum,
        None => 0,
    };

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied boostcount from call: [{}]", boostcount);

    if endex > 0 {
        println!("** Supplied endex from call: [{}]", endex);
    }

    //Get the boosts/streams/sent from db for returning
    let results;

    if list == "streams" {
        results = dbif::get_streams_from_db(&state.helipad_config.database_file_path, index, boostcount, old, false);
    }
    else if list == "sent" {
        results = dbif::get_payments_from_db(&state.helipad_config.database_file_path, index, boostcount, old, false);
    }
    else { // boosts
        results = dbif::get_boosts_from_db(&state.helipad_config.database_file_path, index, boostcount, old, false);
    }

    match results {
        Ok(boosts) => {
            let mut csv = String::new();

            //CSV column name header
            csv.push_str(format!("count,index,time,value_msat,value_sat,value_msat_total,value_sat_total,action,sender,app,message,podcast,episode,remote_podcast,remote_episode\n").as_str());

            //Iterate the boost set
            let mut count: u64 = 1;
            for boost in boosts {
                //Parse out a friendly date
                let dt = DateTime::from_timestamp(boost.time, 0).expect(&format!("Unable to parse boost time: {}", boost.time));
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

            return (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string()),
                    (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.csv\"", list))
                ],
                csv
            ).into_response();
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting boosts.").into_response();
        }
    }
}