// use crate::{Context, Request, Body, Response};
use axum::{
    extract::{Form, Path, Query, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{Html, Json, Redirect, IntoResponse, Response},
};

use axum_extra::{
    extract::cookie::{CookieJar, Cookie},
};

// use axum_macros::debug_handler;
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};

use chrono::{DateTime, TimeDelta, Utc};
use crate::{AppState, lightning, podcastindex};
use dbif::{BoostRecord, BoostFilters, NumerologyRecord, WebhookRecord};
use handlebars::{Handlebars, JsonRender};
use jsonwebtoken::{decode, encode, Algorithm, Header, DecodingKey, EncodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{fs, str};
use std::string::String;
use url::Url;
use tempfile::NamedTempFile;
use std::collections::HashMap;
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

    let token = match decode::<JwtClaims>(jwt, &DecodingKey::from_secret(secret.as_ref()), &Validation::new(Algorithm::HS256)) {
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
    if path.starts_with("/api/v1") || ctype.starts_with("application/json") {
        return (StatusCode::FORBIDDEN, "Not logged in").into_response(); // json response
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
            Err(e) => {
                eprintln!("** Unable to open template file {}: {}.\n", self.0, e);
                return (StatusCode::BAD_REQUEST, "Unable to open template file").into_response();
            }
        };

        let doc_rendered = match reg.render_template(&doc, &self.1) {
            Ok(rendered) => rendered,
            Err(e) => {
                eprintln!("** Unable to render template {}: {}.\n", self.0, e);
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


//Numerology definitions file
pub async fn numerology_json(State(state): State<AppState>) -> Response {
    let results = dbif::get_numerology_from_db(&state.helipad_config.database_file_path).unwrap();
    Json(results).into_response()
}

//API - give back node info
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

pub async fn api_v1_settings(State(state): State<AppState>) -> Response {
    match dbif::load_settings_from_db(&state.helipad_config.database_file_path) {
        Ok(settings) => {
            Json(settings).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting settings: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting settings.").into_response()
        }
    }
}

//API - give back the node balance
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
#[derive(Debug, Serialize, Deserialize)]
pub struct BoostParams {
    index: u64,
    count: u64,
    old: Option<bool>,
    podcast: Option<String>,
}

impl Default for BoostParams {
    fn default() -> Self {
        Self {
            index: 0,
            count: 0,
            old: Some(false),
            podcast: None,
        }
    }
}

pub async fn api_v1_boosts(
    Query(params): Query<BoostParams>,
    State(state): State<AppState>,
) -> Response {

    let index = params.index;
    let boostcount = params.count;

    //Was the "old" flag used?
    let old = params.old.is_some();

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied boost count from call: [{}]", boostcount);

    let mut filters = BoostFilters::new();
    filters.podcast = params.podcast;

    //Get the boosts from db for returning
    match dbif::get_boosts_from_db(&state.helipad_config.database_file_path, index, boostcount, old, true, filters) {
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
pub async fn api_v1_streams(
    params: Option<Query<BoostParams>>,
    State(state): State<AppState>
) -> Response {
    let Query(params) = params.unwrap_or_default();

    let index = params.index;
    let boostcount = params.count;

    //Was the "old" flag used?
    let old = params.old.is_some();

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied stream count from call: [{}]", boostcount);

    let mut filters = BoostFilters::new();
    filters.podcast = params.podcast;

    //Get the boosts from db for returning
    match dbif::get_streams_from_db(&state.helipad_config.database_file_path, index, boostcount, old, true, filters) {
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
pub async fn api_v1_sent(
    params: Option<Query<BoostParams>>,
    State(state): State<AppState>
) -> Response {
    let Query(params) = params.unwrap_or_default();

    let index = params.index;
    let boostcount = params.count;

    //Was the "old" flag used?
    let old = params.old.is_some();

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied sent boost count from call: [{}]", boostcount);

    let mut filters = BoostFilters::new();
    filters.podcast = params.podcast;

    //Get sent boosts from db for returning
    match dbif::get_payments_from_db(&state.helipad_config.database_file_path, index, boostcount, old, true, filters) {
        Ok(sent_boosts) => {
            Json(sent_boosts).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting sent boosts: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting sent boosts.").into_response()
        }
    }
}

pub async fn api_v1_podcasts(
    State(state): State<AppState>
) -> Response {

    //Get the podcasts from db for returning
    match dbif::get_podcasts_from_db(&state.helipad_config.database_file_path) {
        Ok(podcasts) => {
            Json(podcasts).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting podcasts: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting podcasts.").into_response()
        }
    }
}

pub async fn api_v1_sent_podcasts(
    State(state): State<AppState>
) -> Response {

    //Get the podcasts from db for returning
    match dbif::get_sent_podcasts_from_db(&state.helipad_config.database_file_path) {
        Ok(podcasts) => {
            Json(podcasts).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting sent podcasts: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting sent podcasts.").into_response()
        }
    }
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

    let boost = match dbif::get_single_invoice_from_db(&state.helipad_config.database_file_path, "", index, true) {
        Ok(Some(boost)) => boost,
        Ok(None) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Unknown boost index.").into_response();
        },
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error finding boost index.").into_response();
        }
    };

    let tlv = boost.parse_tlv().unwrap();

    let pub_key = tlv["reply_address"].as_str().unwrap_or_default().to_string();

    let custom_key = match tlv["reply_custom_key"].as_u64() {
        None => None,
        Some(0) => None,
        Some(rck) => Some(rck),
    };

    let custom_value = match tlv["reply_custom_value"].as_str() {
        None => None,
        Some("") => None,
        Some(rcv) => Some(rcv.to_string()),
    };

    if pub_key.is_empty() {
        return (StatusCode::BAD_REQUEST, "** No reply_address found in boost").into_response();
    }

    if custom_key.is_none() && custom_value.is_some() {
        return (StatusCode::BAD_REQUEST, "** No reply_custom_key found in boost").into_response();
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
    let lightning = match lightning::connect_to_lnd(&helipad_config.node_address, &helipad_config.cert_path, &helipad_config.macaroon_path).await {
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
    let webhooks = match dbif::get_webhooks_from_db(db_filepath, None) {
        Ok(wh) => wh,
        Err(e) => {
            eprintln!("** Error getting webhooks: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting webhooks.".to_string()).into_response();
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

pub async fn webhook_settings_load(
    Path(idx): Path<String>,
    State(state): State<AppState>
) -> Response {

    let index = match idx.as_str() {
        "add" => 0,
        idx => idx.parse().unwrap(),
    };

    let webhook = match index {
        0 => None,
        _ => match dbif::load_webhook_from_db(&state.helipad_config.database_file_path, index) {
            Ok(wh) => Some(wh),
            Err(e) => {
                eprintln!("** Error loading webhook: {}.\n", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "** Error loading webhook.".to_string()).into_response();
            }
        }
    };

    let equality = match webhook.clone() {
        Some(wh) => wh.equality,
        None => "".to_string(),
    };

    let params = json!({
        "webhook": webhook,
        "equality": json!({
            "any": equality.is_empty(),
            "eq": equality == "=",
            "in": equality == "=~",
            "lt": equality == "<",
            "gte": equality == ">=",
        }),
    });

    println!("** load_webhook_from_db({})", index);

    HtmlTemplate("webroot/template/webhook-edit.hbs", params).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookSaveForm {
    url: String,
    token: String,
    on_boost: Option<bool>,
    on_stream: Option<bool>,
    on_sent: Option<bool>,
    equality: Option<String>,
    amount: Option<String>,
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

    let mut equality = form.equality.unwrap_or_default();
    let mut amount: u64 = form.amount.unwrap_or_default().parse().unwrap_or_default();

    if equality.is_empty() || amount == 0 {
        equality = String::new();
        amount = 0;
    }

    let webhook = WebhookRecord {
        index,
        url: form.url,
        token: form.token,
        on_boost: form.on_boost.unwrap_or(false),
        on_stream: form.on_stream.unwrap_or(false),
        on_sent: form.on_sent.unwrap_or(false),
        equality,
        amount,
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

pub async fn general_settings_load(State(state): State<AppState>) -> impl IntoResponse {
    let settings = dbif::load_settings_from_db(&state.helipad_config.database_file_path).unwrap();
    HtmlTemplate("webroot/template/general-settings.hbs", json!({"settings": settings}))
}

#[derive(Debug, TryFromMultipart)]
pub struct GeneralSettingsMultipart {
    show_received_sats: Option<bool>,
    show_split_percentage: Option<bool>,
    hide_boosts: Option<bool>,
    hide_boosts_below: Option<String>,
    play_pew: Option<bool>,
    resolve_nostr_refs: Option<bool>,
    show_hosted_wallet_ids: Option<bool>,

    // The `unlimited arguments` means that this field will be limited to the
    // total size of the request body. If you want to limit the size of this
    // field to a specific value you can also specify a limit in bytes, like
    // '5MiB' or '1GiB'.
    #[form_data(limit = "5MiB")]
    custom_pew_file: Option<FieldData<NamedTempFile>>,
    custom_pew_existing: Option<bool>,
}

pub async fn general_settings_save(
    State(state): State<AppState>,
    TypedMultipart(parts): TypedMultipart<GeneralSettingsMultipart>,
) -> impl IntoResponse {

    let hide_boosts_below = match parts.hide_boosts_below {
        Some(s) => match s.is_empty() {
            false => Some(s.parse::<u64>().unwrap_or(0)),
            true => None,
        },
        None => None,
    };

    let mut settings = dbif::load_settings_from_db(&state.helipad_config.database_file_path).unwrap();

    settings.show_received_sats = parts.show_received_sats.unwrap_or(false);
    settings.show_split_percentage = parts.show_split_percentage.unwrap_or(false);
    settings.hide_boosts = parts.hide_boosts.unwrap_or(false);
    settings.hide_boosts_below = hide_boosts_below;
    settings.play_pew = parts.play_pew.unwrap_or(false);
    settings.resolve_nostr_refs = parts.resolve_nostr_refs.unwrap_or(false);
    settings.show_hosted_wallet_ids = parts.show_hosted_wallet_ids.unwrap_or(false);

    if !settings.hide_boosts {
        settings.hide_boosts_below = None;
    }

    if let Some(field) = parts.custom_pew_file {
        let from_path = field.contents.path();
        let to_path =  format!("{}/custom_pew.mp3", state.helipad_config.sound_path);
        let bytes = std::fs::copy(from_path, &to_path).unwrap_or(0);

        if bytes > 0 {
            println!("** Wrote custom pew to: {}", to_path);
            settings.custom_pew_file = Some("custom_pew.mp3".to_string())
        } else {
            settings.custom_pew_file = None;
        }
    } else if parts.custom_pew_existing.is_none() {
        settings.custom_pew_file = None;
    }

    dbif::save_settings_to_db(&state.helipad_config.database_file_path, &settings).unwrap();

    HtmlTemplate("webroot/template/general-settings.hbs", json!({"settings": settings, "saved": true}))
}

pub fn numerology_list(db_filepath: &String) -> impl IntoResponse {
    let results = dbif::get_numerology_from_db(db_filepath).unwrap();
    HtmlTemplate("webroot/template/numerology-list.hbs", json!({"numerology": results}))
}

pub async fn numerology_settings_list(State(state): State<AppState>) -> impl IntoResponse {
    numerology_list(&state.helipad_config.database_file_path)
}

pub async fn numerology_settings_load(
    State(state): State<AppState>,
    Path(idx): Path<String>,
) -> impl IntoResponse {

    let index = match idx.as_str() {
        "add" => 0,
        idx => idx.parse().unwrap(),
    };

    let result = if index > 0 {
        dbif::load_numerology_from_db(&state.helipad_config.database_file_path, index).ok()
    } else {
        None
    };

    let equality = match &result {
        Some(eq) => eq.equality.clone(),
        None => "".to_string(),
    };

    let params = json!({
        "numerology": result,
        "equality": json!({
            "eq": equality == "=",
            "in": equality == "=~",
            "lt": equality == "<",
            "gte": equality == ">=",
        }),
    });

    HtmlTemplate("webroot/template/numerology-edit.hbs", params)
}

#[derive(Debug, TryFromMultipart)]
pub struct NumerologyMultipart {
    position: u64,
    amount: u64,
    equality: String,
    emoji: Option<String>,

    #[form_data(limit = "5MiB")]
    sound_file: Option<FieldData<NamedTempFile>>,
    sound_file_existing: Option<bool>,

    description: Option<String>,
}

pub async fn numerology_settings_save(
    State(state): State<AppState>,
    Path(idx): Path<String>,
    TypedMultipart(parts): TypedMultipart<NumerologyMultipart>,
) -> Response {
    let db_filepath = state.helipad_config.database_file_path;

    let index = match idx.as_str() {
        "add" => 0,
        idx => idx.parse().unwrap(),
    };

    let mut numero = NumerologyRecord {
        index,
        position: parts.position,
        amount: parts.amount,
        equality: parts.equality,
        emoji: parts.emoji,
        sound_file: None,
        description: parts.description,
    };

    if index > 0 {
        let existing = match dbif::load_numerology_from_db(&db_filepath, index) {
            Ok(exist) => exist,
            Err(e) => {
                eprintln!("** Error loading numerology: {}.\n", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "** Error loading numerology.".to_string()).into_response();
            }
        };

        numero.sound_file = existing.sound_file;
    }

    if let Some(field) = parts.sound_file {
        let filename = format!("{}.mp3", parts.amount);
        let from_path = field.contents.path();
        let to_path = format!("{}/{}", state.helipad_config.sound_path, filename);
        let bytes = std::fs::copy(from_path, &to_path).unwrap_or(0);

        if bytes > 0 {
            println!("** Wrote sound file to: {}", to_path);
            numero.sound_file = Some(filename)
        } else {
            numero.sound_file = None;
        }
    } else if parts.sound_file_existing.is_none() {
        numero.sound_file = None;
    }

    let idx = match dbif::save_numerology_to_db(&db_filepath, &numero) {
        Ok(idx) => idx,
        Err(e) => {
            eprintln!("** Error saving numerology: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error saving numerology.").into_response();
        }
    };

    println!("** numerology_settings_save({})", idx);

    numerology_list(&db_filepath).into_response()
}

pub async fn numerology_settings_delete(
    State(state): State<AppState>,
    Path(idx): Path<String>
) -> impl IntoResponse {

    let index = idx.parse().unwrap();

    if let Err(e) = dbif::delete_numerology_from_db(&state.helipad_config.database_file_path, index) {
        eprintln!("** Error deleting numerology: {}.\n", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "** Error deleting numerology.");
    }

    println!("** numerology_settings_delete({})", index);

    (StatusCode::OK, "")
}

pub async fn numerology_settings_reset() -> impl IntoResponse {
    HtmlTemplate("webroot/template/numerology-reset.hbs", "")
}

pub async fn numerology_settings_do_reset(State(state): State<AppState>) -> Response {
    let db_filepath = state.helipad_config.database_file_path;

    if let Err(e) = dbif::reset_numerology_in_db(&db_filepath) {
        eprintln!("** Error resetting numerology: {}.\n", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "** Error resetting numerology.").into_response()
    }

    numerology_list(&db_filepath).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NumerologyPatchForm {
    position: u64,
}

pub async fn numerology_settings_patch(
    State(state): State<AppState>,
    Path(idx): Path<String>,
    Form(params): Form<NumerologyPatchForm>,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let db_filepath = state.helipad_config.database_file_path;

    let index = idx.parse().unwrap();

    let mut numero = match dbif::load_numerology_from_db(&db_filepath, index) {
        Ok(num) => num,
        Err(e) => {
            eprintln!("** Error loading numerology item: {}.\n", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "** Error loading numerology item."));
        }
    };

    numero.position = params.position;

    match dbif::save_numerology_to_db(&db_filepath, &numero) {
        Ok(num) => num,
        Err(e) => {
            eprintln!("** Error saving numerology item: {}.\n", e);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "** Error saving numerology item."));
        }
    };

    println!("** numerology_settings_patch({})", index);

    Ok(numerology_list(&db_filepath))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReportGenerateForm {
    list_boosts: Option<bool>,
    list_streams: Option<bool>,
    list_sent: Option<bool>,
    podcast: String,
    start_date: Option<u64>,
    end_date: Option<u64>,
    include_usd: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BtcPrices {
    bpi: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockchainResponse {
    values: Vec<BlockchainDataPoint>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockchainDataPoint {
    x: i64,  // timestamp
    y: f64,  // price value
}

pub async fn fetch_btc_prices(start_date: u64, end_date: u64) -> Result<Option<BtcPrices>, reqwest::Error> {
    println!("** Fetching BTC prices from {} to {}", start_date, end_date);

    // Calculate time span in days
    let time_diff = end_date - start_date;
    let days = (time_diff / 86400) + 1; // Convert seconds to days and add 1 to include end date
    let timespan = format!("{}days", days);

    let client = reqwest::Client::new();

    // Use Blockchain.com's market price chart API
    let response = client.get("https://api.blockchain.info/charts/market-price")
        .query(&[
            ("timespan", timespan.as_str()),
            ("format", "json")
        ])
        .send()
        .await?;

    let blockchain_data = response.json::<BlockchainResponse>().await?;

    println!("** Received {} data points from Blockchain.com", blockchain_data.values.len());

    // Convert Blockchain.com data to the format expected by the application
    let mut bpi = HashMap::new();

    for point in &blockchain_data.values {
        let timestamp = point.x;

        // Only include points within our requested range
        if timestamp >= start_date as i64 && timestamp <= end_date as i64 {
            if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
                let date = dt.format("%Y-%m-%d").to_string();
                bpi.insert(date, point.y);
            }
        }
    }

    if bpi.is_empty() {
        println!("** No BTC price data found in the requested date range");
        return Ok(None);
    }

    println!("** Processed {} daily price points", bpi.len());
    Ok(Some(BtcPrices { bpi }))
}

pub async fn report_generate(
    State(state): State<AppState>,
    Form(form): Form<ReportGenerateForm>,
) -> impl IntoResponse {
    println!("** report_generate({:#?})", form.clone());

    let mut lists = Vec::new();

    if form.list_boosts.is_some() {
        lists.push("boost");
    }

    if form.list_streams.is_some() {
        lists.push("stream");
    }

    if form.list_sent.is_some() {
        lists.push("sent");
    }

    let mut filters = BoostFilters::new();

    if !form.podcast.is_empty() {
        filters.podcast = Some(form.podcast);
    }

    if let Some(val) = form.start_date {
        filters.start_date = Some(val);
    }

    if let Some(val) = form.end_date {
        filters.end_date = Some(val);
    }

    let mut btc_prices = None;

    if form.include_usd.is_some() && form.start_date.is_some() && form.end_date.is_some() {
        let prices = fetch_btc_prices(form.start_date.unwrap(), form.end_date.unwrap()).await;

        if let Err(e) = prices {
            eprintln!("** Error getting btc prices: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting btc prices.").into_response();
        }

        btc_prices = prices.unwrap();
    }

    let index = 0;
    let boostcount = 0;
    let direction = false;

    let mut csv = String::new();

    //CSV column name header
    let mut headers = "index,type,time,timezone,value_msat,value_sat,value_msat_total,value_sat_total,action,sender,app,message,podcast,episode,remote_podcast,remote_episode,custom_key,custom_value".to_string();

    if btc_prices.is_some() {
        headers.push_str(",btc_close,value_usd,value_usd_total");
    }

    csv.push_str(&headers);
    csv.push('\n');

    for list in lists {
        let results;

        if list == "boost" {
            results = dbif::get_boosts_from_db(&state.helipad_config.database_file_path, index, boostcount, direction, false, filters.clone());
        }
        else if list == "stream" {
            results = dbif::get_streams_from_db(&state.helipad_config.database_file_path, index, boostcount, direction, false, filters.clone());
        }
        else if list == "sent" {
            results = dbif::get_payments_from_db(&state.helipad_config.database_file_path, index, boostcount, direction, false, filters.clone());
        }
        else {
            continue;
        }

        if let Err(e) = results {
            eprintln!("** Error getting boosts: {}.\n", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting boosts.").into_response();
        }

        let boosts = results.unwrap();

        //Iterate the boost set
        for boost in boosts {
            //Parse out a friendly date
            let dt = DateTime::from_timestamp(boost.time, 0).unwrap_or_else(|| panic!("Unable to parse boost time: {}", boost.time));
            let boost_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            let boost_timezone = dt.format("%Z").to_string();

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
                    "{},{},\"{}\",\"{}\",{},{},{},{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"",
                    boost.index,
                    list,
                    boost_time,
                    boost_timezone,
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
                    BoostRecord::escape_for_csv(boost.remote_episode.unwrap_or("".to_string())),
                    BoostRecord::escape_for_csv(boost.custom_key.map(|k| k.to_string()).unwrap_or_default()),
                    BoostRecord::escape_for_csv(boost.custom_value.unwrap_or("".to_string()))
                ).as_str()
            );

            //Include BTC/USD conversion if set
            if let Some(btc_prices) = &btc_prices {
                let date = dt.format("%Y-%m-%d").to_string();

                if let Some(btc_price) = btc_prices.bpi.get(&date) {
                    let sat_price = btc_price / 100_000_000.0;
                    let value_usd = (value_sat as f64) * sat_price;
                    let value_usd_total = (value_sat_total as f64) * sat_price;

                    csv.push_str(format!(",{},{},{}", btc_price, value_usd, value_usd_total).as_str());
                } else {
                    csv.push_str(",,,");
                }
            }

            csv.push('\n');
        }
    }

    // return csv
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"report.csv\"".to_string())
        ],
        csv
    ).into_response()
}

pub async fn report_podcasts_list(State(state): State<AppState>) -> impl IntoResponse {
    match dbif::get_podcasts_from_db(&state.helipad_config.database_file_path) {
        Ok(podcasts) => {
             HtmlTemplate("webroot/template/report-podcasts-list.hbs", json!({"podcasts": podcasts}))
        },
        Err(err) => {
             HtmlTemplate("webroot/template/report-podcasts-list.hbs", json!({"error": err.to_string()}))
        }
    }
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
    let old = params.old.is_some();

    //Was a stop index given?
    let endex = params.end.unwrap_or(0);

    println!("** Supplied index from call: [{}]", index);
    println!("** Supplied boostcount from call: [{}]", boostcount);

    if endex > 0 {
        println!("** Supplied endex from call: [{}]", endex);
    }

    //Get the boosts/streams/sent from db for returning
    let results;
    let filters = BoostFilters::new();

    if list == "streams" {
        results = dbif::get_streams_from_db(&state.helipad_config.database_file_path, index, boostcount, old, false, filters);
    }
    else if list == "sent" {
        results = dbif::get_payments_from_db(&state.helipad_config.database_file_path, index, boostcount, old, false, filters);
    }
    else { // boosts
        results = dbif::get_boosts_from_db(&state.helipad_config.database_file_path, index, boostcount, old, false, filters);
    }

    match results {
        Ok(boosts) => {
            let mut csv = String::new();

            //CSV column name header
            csv.push_str("count,index,time,timezone,value_msat,value_sat,value_msat_total,value_sat_total,action,sender,app,message,podcast,episode,remote_podcast,remote_episode,custom_key,custom_value\n");

            //Iterate the boost set
            let mut count: u64 = 1;
            for boost in boosts {
                //Parse out a friendly date
                let dt = DateTime::from_timestamp(boost.time, 0).unwrap_or_else(|| panic!("Unable to parse boost time: {}", boost.time));
                let boost_time = dt.format("%Y-%m-%d %H:%M:%S").to_string();
                let boost_timezone = dt.format("%Z").to_string();

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
                        "{},{},\"{}\",\"{}\",{},{},{},{},{},\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{}\"\n",
                        count,
                        boost.index,
                        boost_time,
                        boost_timezone,
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
                        BoostRecord::escape_for_csv(boost.remote_episode.unwrap_or("".to_string())),
                        BoostRecord::escape_for_csv(boost.custom_key.map(|k| k.to_string()).unwrap_or_default()),
                        BoostRecord::escape_for_csv(boost.custom_value.unwrap_or("".to_string()))
                    ).as_str()
                );

                //Keep count
                count += 1;

                //If an exit point was given then bail when it's reached
                if (old && boost.index <= endex) || (!old && boost.index >= endex) {
                    break;
                }
            }

            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/plain; charset=utf-8".to_string()),
                    (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}.csv\"", list))
                ],
                csv
            ).into_response()
        }
        Err(e) => {
            eprintln!("** Error getting boosts: {}.\n", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "** Error getting boosts.").into_response()
        }
    }
}