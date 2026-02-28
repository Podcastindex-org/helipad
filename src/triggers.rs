use anyhow::{Context, Result};
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use reqwest::redirect::Policy;
use rosc::{encoder, OscMessage, OscPacket, OscType};
use serde::Serialize;
use std::net::UdpSocket;
use serde_json::json;

/// Represents a boost record with associated client and server effects
#[derive(Serialize, Clone, Debug)]
pub struct BoostWithEffects {
    #[serde(flatten)]
    pub boost: dbif::BoostRecord,
    pub effects: Vec<ClientEffect>,
    #[serde(skip_serializing)]
    pub server_effects: Vec<ServerEffect>,
}

/// Payload sent to webhooks
#[derive(Serialize)]
pub struct WebhookPayload {
    pub direction: String,
    #[serde(flatten)]
    pub boost: dbif::BoostRecord,
}

/// MIDI effect configuration
#[derive(Serialize, Clone, Debug)]
pub struct MidiEffect {
    pub note: u8,
    pub velocity: u8,
    pub channel: u8,
    pub duration: u16,
}

/// Sound effect configuration
#[derive(Serialize, Clone, Debug)]
pub struct SoundEffect {
    pub sound_file: String,
    pub sound_name: String,
}

/// Client-side effects (MIDI, sounds)
#[derive(Serialize, Clone, Debug)]
pub struct ClientEffect {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi: Option<MidiEffect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<SoundEffect>,
}

impl Default for ClientEffect {
    fn default() -> Self {
        Self {
            midi: None,
            sound: None,
        }
    }
}

/// Webhook effect configuration
#[derive(Serialize, Clone, Debug)]
pub struct WebhookEffect {
    pub index: u64,
    pub url: String,
    pub token: String,
}

/// OSC (Open Sound Control) effect configuration
#[derive(Serialize, Clone, Debug)]
pub struct OscEffect {
    pub index: u64,
    pub address: String,
    pub port: u16,
    pub path: String,
    pub args: String,
}

/// Server-side effects (webhooks, OSC)
#[derive(Serialize, Clone, Debug)]
pub struct ServerEffect {
    pub webhook: Option<WebhookEffect>,
    pub osc: Option<OscEffect>,
}

impl Default for ServerEffect {
    fn default() -> Self {
        Self {
            webhook: None,
            osc: None,
        }
    }
}

/// Processes triggers for a boost, finding matching triggers and running server-side effects
pub async fn process_triggers(
    db_filepath: &str,
    boost: &dbif::BoostRecord,
) -> Result<BoostWithEffects> {
    let boost_with_effects = get_boost_with_effects(db_filepath, boost.clone())
        .await
        .context("Unable to get boost with triggers")?;

    run_server_effects(db_filepath, &boost_with_effects)
        .await
        .context("Unable to run server effects")?;

    Ok(boost_with_effects)
}

pub async fn test_trigger(db_filepath: &str, trigger: &dbif::TriggerRecord) -> Result<BoostWithEffects> {
    // Create a sample boost record for testing (values don't matter since we skip filter checks)
    let test_msats = 100000i64; // 100 sats default
    let test_boost = dbif::BoostRecord {
        index: 99999,
        time: Utc::now().timestamp(),
        value_msat: test_msats,
        value_msat_total: test_msats,
        action: 2, // boost action
        sender: "Test Sender".to_string(),
        app: "Helipad".to_string(),
        message: "This is a test trigger message".to_string(),
        podcast: "Test Podcast".to_string(),
        episode: "Test Episode".to_string(),
        tlv: json!({
            "action": "boost",
            "app_name": "Helipad",
            "app_version": env!("CARGO_PKG_VERSION"),
            "podcast": "Test Podcast",
            "episode": "Test Episode",
            "sender_name": "Test Sender",
            "message": "This is a test trigger message",
            "value_msat": test_msats,
            "value_msat_total": test_msats
        }).to_string(),
        remote_podcast: None,
        remote_episode: None,
        reply_sent: false,
        custom_key: None,
        custom_value: None,
        memo: None,
        payment_info: None,
    };

    let server_effects = match get_server_effect(&trigger) {
        Some(effect) => vec![effect],
        None => Vec::new(),
    };

    let client_effects = match get_client_effect(&trigger) {
        Some(effect) => vec![effect],
        None => Vec::new(),
    };

    let boost_with_effects = BoostWithEffects {
        boost: test_boost,
        effects: client_effects,
        server_effects: server_effects,
    };

    run_server_effects(db_filepath, &boost_with_effects).await?;

    Ok(boost_with_effects)
}

/// Gets multiple boosts with their associated trigger effects
pub async fn get_boosts_with_triggers(
    db_filepath: &str,
    boosts: Vec<dbif::BoostRecord>,
) -> Result<Vec<BoostWithEffects>> {
    let triggers = dbif::get_triggers_from_db(db_filepath)
        .map_err(|e| anyhow::anyhow!("Unable to get triggers from database: {}", e))?;

    let default_sound = get_default_sound_effect(db_filepath)?;

    let boosts_with_triggers = boosts
        .into_iter()
        .map(|boost| map_boost_with_effects(&triggers, &boost, &default_sound))
        .collect();

    Ok(boosts_with_triggers)
}

/// Get a single boost with its associated trigger effects
fn map_boost_with_effects(
    triggers: &[dbif::TriggerRecord],
    boost: &dbif::BoostRecord,
    default_sound: &Option<SoundEffect>,
) -> BoostWithEffects {
    let mut effects = Vec::new();
    let mut server_effects = Vec::new();

    for trigger in triggers.iter().filter(|t| t.enabled) {
        if !matches_trigger_on_condition(trigger, boost) {
            continue;
        }

        if !matches_filter_conditions(trigger, boost) {
            continue;
        }

        if let Some(client_effect) = get_client_effect(trigger) {
            effects.push(client_effect);
        }

        if let Some(server_effect) = get_server_effect(trigger) {
            server_effects.push(server_effect);
        }
    }

    // Use default sound if no effects were triggered
    let has_sound = effects.iter().any(|e| e.sound.is_some());

    if !has_sound && default_sound.is_some() {
        effects.push(ClientEffect {
            sound: default_sound.clone(),
            midi: None,
        });
    }

    BoostWithEffects {
        boost: boost.clone(),
        effects,
        server_effects,
    }
}

/// Gets a single boost with its associated trigger effects
pub async fn get_boost_with_effects(
    db_filepath: &str,
    boost: dbif::BoostRecord,
) -> Result<BoostWithEffects> {
    let result = get_boosts_with_triggers(db_filepath, vec![boost])
        .await
        .context("Unable to get boosts with triggers")?;

    result
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No boosts with triggers found"))
}

/// Checks if a trigger matches the boost's action type and direction
fn matches_trigger_on_condition(trigger: &dbif::TriggerRecord, boost: &dbif::BoostRecord) -> bool {
    // Check if trigger matches sent/received direction
    if boost.payment_info.is_some() && !trigger.on_sent {
        return false; // This is a sent boost but trigger doesn't match sent
    }

    // Check if trigger matches the action type
    match boost.action_name().as_str() {
        "stream" if !trigger.on_stream => false,
        "boost" if !trigger.on_boost => false,
        "auto" if !trigger.on_auto => false,
        "invoice" if !trigger.on_invoice => false,
        _ => true,
    }
}

/// Checks if the boost matches all filter conditions of the trigger
fn matches_filter_conditions(trigger: &dbif::TriggerRecord, boost: &dbif::BoostRecord) -> bool {
    // Convert msat to sats (clamped to non-negative values)
    let sats = (boost.value_msat_total.max(0) / 1000) as u64;

    matches_filter_condition(&trigger.amount_equality, &trigger.amount, &sats)
        && matches_filter_condition(&trigger.sender_equality, &trigger.sender, &boost.sender)
        && matches_filter_condition(&trigger.app_equality, &trigger.app, &boost.app)
        && matches_filter_condition(&trigger.podcast_equality, &trigger.podcast, &boost.podcast)
}

/// Matches a single filter condition against a value
fn matches_filter_condition<T>(
    equality: &Option<String>,
    filter_value: &Option<T>,
    actual_value: &T,
) -> bool
where
    T: PartialEq + PartialOrd + std::fmt::Display,
{
    let equality = match equality {
        Some(eq) => eq,
        None => return true,
    };

    let filter_value = match filter_value {
        Some(val) => val,
        None => return true,
    };

    match equality.as_str() {
        ">=" => actual_value >= filter_value,
        "<" => actual_value < filter_value,
        "=" => actual_value == filter_value,
        "!=" => actual_value != filter_value,
        "=~" => actual_value.to_string().contains(&filter_value.to_string()),
        "^=" => actual_value.to_string().starts_with(&filter_value.to_string()),
        "$=" => actual_value.to_string().ends_with(&filter_value.to_string()),
        _ => false,
    }
}

/// Gets the default sound effect from settings
fn get_default_sound_effect(db_filepath: &str) -> Result<Option<SoundEffect>> {
    let settings = dbif::load_settings_from_db(db_filepath)
        .map_err(|e| anyhow::anyhow!("Unable to load settings from database: {}", e))?;

    if !settings.play_pew {
        return Ok(None);
    }

    let sound_file = settings
        .custom_pew_file
        .map(|file| format!("sound/{}", file))
        .unwrap_or_else(|| "pew.mp3".to_string());

    Ok(Some(SoundEffect {
        sound_file: sound_file.clone(),
        sound_name: sound_file,
    }))
}

/// Builds client effect (MIDI and/or sound) from trigger configuration
pub fn get_client_effect(trigger: &dbif::TriggerRecord) -> Option<ClientEffect> {
    let midi = get_midi(trigger);
    let sound = get_sound(trigger);

    if midi.is_some() || sound.is_some() {
        Some(ClientEffect {
            midi,
            sound,
        })
    }
    else {
        None
    }
}

/// Builds server effects (webhook and/or OSC) from trigger configuration
pub fn get_server_effect(trigger: &dbif::TriggerRecord) -> Option<ServerEffect> {
    let webhook = trigger.webhook_url.as_ref().map(|url| WebhookEffect {
        index: trigger.index,
        url: url.clone(),
        token: trigger.webhook_token.clone().unwrap_or_default(),
    });

    let osc =
        match (&trigger.osc_address, &trigger.osc_port, &trigger.osc_path) {
            (Some(address), Some(port), Some(path)) => Some(OscEffect {
                index: trigger.index,
                address: address.clone(),
                port: *port,
                path: path.clone(),
                args: trigger.osc_args.clone().unwrap_or_default(),
            }),
            _ => None,
        };

    if webhook.is_some() || osc.is_some() {
        Some(ServerEffect {
            webhook,
            osc,
        })
    }
    else {
        None
    }
}

/// Executes all server-side effects (webhooks, OSC) for a boost
pub async fn run_server_effects(
    db_filepath: &str,
    boost_with_effects: &BoostWithEffects,
) -> Result<()> {
    for effect in &boost_with_effects.server_effects {
        if let Some(webhook) = &effect.webhook {
            handle_webhook_effect(db_filepath, webhook, boost_with_effects).await;
        }

        if let Some(osc) = &effect.osc {
            handle_osc_effect(db_filepath, osc).await;
        }
    }

    Ok(())
}

/// Handles sending a webhook and updating its status
async fn handle_webhook_effect(
    db_filepath: &str,
    webhook: &WebhookEffect,
    boost_with_effects: &BoostWithEffects,
) {
    let timestamp = Utc::now().timestamp();

    match send_webhook(webhook, boost_with_effects).await {
        Ok(successful) => {
            if let Err(e) =
                dbif::set_trigger_webhook_last_request(db_filepath, webhook.index, successful, timestamp)
            {
                eprintln!("Error setting trigger webhook last request status: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Error sending webhook: {}", e);
        }
    }
}

/// Handles sending an OSC message and updating its status
async fn handle_osc_effect(db_filepath: &str, osc: &OscEffect) {
    let timestamp = Utc::now().timestamp();

    match send_osc(osc).await {
        Ok(successful) => {
            if let Err(e) = dbif::set_trigger_osc_last_request(db_filepath, osc.index, successful, timestamp) {
                eprintln!("Error setting trigger OSC last request status: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Error sending OSC message: {}", e);
        }
    }
}

/// Builds MIDI effect from trigger configuration
fn get_midi(trigger: &dbif::TriggerRecord) -> Option<MidiEffect> {
    if let Some(midi_note) = trigger.midi_note {
        Some(MidiEffect {
            note: midi_note,
            velocity: trigger.midi_velocity.unwrap_or(100),
            channel: trigger.midi_channel.unwrap_or(1),
            duration: trigger.midi_duration.unwrap_or(500),
        })
    }
    else {
        None
    }
}

/// Builds sound effect from trigger configuration
fn get_sound(trigger: &dbif::TriggerRecord) -> Option<SoundEffect> {
    trigger.sound_file.as_ref().map(|sound| {
        let sound_file = format!("sound/{}", sound);
        let sound_name = sound.clone();

        SoundEffect {
            sound_file,
            sound_name,
        }
    })
}

/// Sends an HTTP webhook with boost data
async fn send_webhook(
    effect: &WebhookEffect,
    boost_with_effects: &BoostWithEffects,
) -> Result<bool> {
    let mut headers = HeaderMap::new();

    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    let user_agent = format!("Helipad/{}", env!("CARGO_PKG_VERSION"));
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&user_agent).context("Unable to create user agent header")?,
    );

    if !effect.token.is_empty() {
        let token = format!("Bearer {}", effect.token);
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&token).context("Unable to create authorization header")?,
        );
    }

    let client = reqwest::Client::builder()
        .redirect(Policy::limited(5))
        .build()
        .context("Unable to build reqwest client")?;

    let direction = if boost_with_effects.boost.payment_info.is_some() {
        "outgoing"
    } else {
        "incoming"
    };

    let payload = WebhookPayload {
        direction: direction.to_string(),
        boost: boost_with_effects.boost.clone(),
    };

    let json = serde_json::to_string_pretty(&payload)
        .context("Unable to encode webhook payload as JSON")?;

    let res = client
        .post(&effect.url)
        .body(json)
        .headers(headers)
        .send()
        .await
        .context("Unable to send webhook")?;

    let status = res.status();
    let response = res.text().await;

    let successful = status == 200 && response.is_ok();

    if successful {
        println!("Webhook sent to {}: {}", effect.url, response.unwrap());
    } else if status != 200 {
        eprintln!(
            "Webhook returned {}: {}",
            status,
            response.unwrap_or_default()
        );
    } else if let Err(e) = response {
        eprintln!("Webhook Error: {}", e);
    }

    Ok(successful)
}

/// Sends an OSC (Open Sound Control) message via UDP
async fn send_osc(effect: &OscEffect) -> Result<bool> {
    let args = if !effect.args.is_empty() {
        parse_osc_args(&effect.args)
    } else {
        vec![OscType::Int(1)]
    };

    let msg = OscMessage {
        addr: effect.path.clone(),
        args,
    };

    let packet = OscPacket::Message(msg);
    let encoded = encoder::encode(&packet).context("Unable to encode OSC message")?;

    let socket = UdpSocket::bind("0.0.0.0:0").context("Unable to bind UDP socket")?;
    let dest = format!("{}:{}", effect.address, effect.port);

    match socket.send_to(&encoded, &dest) {
        Ok(_) => {
            println!("OSC message sent to {}: {}", dest, effect.path);
            Ok(true)
        }
        Err(e) => {
            eprintln!("Failed to send OSC message to {}: {}", dest, e);
            Ok(false)
        }
    }
}

/// Parses OSC argument strings into typed OSC arguments
fn parse_osc_args(args: &str) -> Vec<OscType> {
    args
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|value| {
            // Try to parse as integer
            if let Ok(i) = value.parse::<i32>() {
                return OscType::Int(i);
            }

            // Try to parse as float
            if let Ok(f) = value.parse::<f32>() {
                return OscType::Float(f);
            }

            // Check for boolean values
            if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("t") {
                return OscType::Bool(true);
            }

            if value.eq_ignore_ascii_case("false") || value.eq_ignore_ascii_case("f") {
                return OscType::Bool(false);
            }

            // Default to string (remove quotes if present)
            let cleaned = value.trim_matches('"').trim_matches('\'');
            OscType::String(cleaned.to_string())
        })
        .collect()
}
