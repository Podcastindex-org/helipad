use rusqlite::{Connection, params};
use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::{connect_to_database, HydraError};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TriggerRecord {
    pub index: u64,
    pub position: u64,
    pub enabled: bool,
    pub on_boost: bool,
    pub on_stream: bool,
    pub on_auto: bool,
    pub on_sent: bool,
    pub on_invoice: bool,
    pub amount: Option<u64>,
    pub amount_equality: Option<String>,
    pub sender: Option<String>,
    pub sender_equality: Option<String>,
    pub app: Option<String>,
    pub app_equality: Option<String>,
    pub podcast: Option<String>,
    pub podcast_equality: Option<String>,
    pub sound_file: Option<String>,
    pub sound_name: Option<String>,
    pub webhook_url: Option<String>,
    pub webhook_token: Option<String>,
    pub webhook_successful: Option<bool>,
    pub webhook_timestamp: Option<i64>,
    pub osc_address: Option<String>,
    pub osc_port: Option<u16>,
    pub osc_path: Option<String>,
    pub osc_args: Option<String>,
    pub osc_successful: Option<bool>,
    pub osc_timestamp: Option<i64>,
    pub midi_note: Option<u8>,
    pub midi_velocity: Option<u8>,
    pub midi_channel: Option<u8>,
    pub midi_duration: Option<u16>
}

impl Default for TriggerRecord {
    fn default() -> Self {
        Self {
            index: 0,
            position: 0,
            enabled: false,
            on_boost: false,
            on_stream: false,
            on_auto: false,
            on_sent: false,
            on_invoice: false,
            amount: None,
            amount_equality: None,
            sender: None,
            sender_equality: None,
            app: None,
            app_equality: None,
            podcast: None,
            podcast_equality: None,
            sound_file: None,
            sound_name: None,
            webhook_url: None,
            webhook_token: None,
            webhook_successful: None,
            webhook_timestamp: None,
            osc_address: None,
            osc_port: None,
            osc_path: None,
            osc_args: None,
            osc_successful: None,
            osc_timestamp: None,
            midi_note: None,
            midi_velocity: None,
            midi_channel: None,
            midi_duration: None,
        }
    }
}

pub fn create_triggers_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS triggers (
            idx integer primary key,
            position integer not null,
            enabled integer not null,
            on_boost integer not null,
            on_stream integer not null,
            on_auto integer not null,
            on_sent integer not null,
            on_invoice integer not null,
            amount integer,
            amount_equality text,
            sender text,
            sender_equality text,
            app text,
            app_equality text,
            podcast text,
            podcast_equality text,
            sound_file text,
            sound_name text,
            webhook_url text,
            webhook_token text,
            webhook_successful integer,
            webhook_timestamp integer,
            osc_address text,
            osc_port integer,
            osc_path text,
            osc_args text,
            osc_successful integer,
            osc_timestamp integer,
            midi_note integer,
            midi_velocity integer,
            midi_channel integer,
            midi_duration integer
        )",
        [],
    ) {
        eprintln!("{}", e);
        return Err(Box::new(HydraError("Failed to create database triggers table.".into())))
    }

    println!("Triggers table is ready.");
    Ok(true)
}

pub fn get_triggers_from_db(filepath: &str) -> Result<Vec<TriggerRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut results: Vec<TriggerRecord> = Vec::new();

    let mut stmt = conn.prepare(
        r#"SELECT
             idx,
             position,
             enabled,
             on_boost,
             on_stream,
             on_auto,
             on_sent,
             on_invoice,
             amount,
             amount_equality,
             sender,
             sender_equality,
             app,
             app_equality,
             podcast,
             podcast_equality,
             sound_file,
             sound_name,
             webhook_url,
             webhook_token,
             webhook_successful,
             webhook_timestamp,
             osc_address,
             osc_port,
             osc_path,
             osc_args,
             osc_successful,
             osc_timestamp,
             midi_note,
             midi_velocity,
             midi_channel,
             midi_duration
        FROM
            triggers
        ORDER BY
            position
        "#
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TriggerRecord {
            index: row.get(0)?,
            position: row.get(1)?,
            enabled: row.get(2)?,
            on_boost: row.get(3)?,
            on_stream: row.get(4)?,
            on_auto: row.get(5)?,
            on_sent: row.get(6)?,
            on_invoice: row.get(7)?,
            amount: row.get(8)?,
            amount_equality: row.get(9)?,
            sender: row.get(10).ok(),
            sender_equality: row.get(11).ok(),
            app: row.get(12).ok(),
            app_equality: row.get(13).ok(),
            podcast: row.get(14).ok(),
            podcast_equality: row.get(15).ok(),
            sound_file: row.get(16).ok(),
            sound_name: row.get(17).ok(),
            webhook_url: row.get(18).ok(),
            webhook_token: row.get(19).ok(),
            webhook_successful: row.get(20).ok(),
            webhook_timestamp: row.get(21).ok(),
            osc_address: row.get(22).ok(),
            osc_port: row.get(23).ok(),
            osc_path: row.get(24).ok(),
            osc_args: row.get(25).ok(),
            osc_successful: row.get(26).ok(),
            osc_timestamp: row.get(27).ok(),
            midi_note: row.get(28).ok(),
            midi_velocity: row.get(29).ok(),
            midi_channel: row.get(30).ok(),
            midi_duration: row.get(31).ok(),
        })
    }).unwrap();

    for row in rows {
        results.push(row.unwrap());
    }

    Ok(results)
}

pub fn load_trigger_from_db(filepath: &str, index: u64) -> Result<TriggerRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            position,
            enabled,
            on_boost,
            on_stream,
            on_auto,
            on_sent,
            on_invoice,
            amount,
            amount_equality,
            sender,
            sender_equality,
            app,
            app_equality,
            podcast,
            podcast_equality,
            sound_file,
            sound_name,
            webhook_url,
            webhook_token,
            webhook_successful,
            webhook_timestamp,
            osc_address,
            osc_port,
            osc_path,
            osc_args,
            osc_successful,
            osc_timestamp,
            midi_note,
            midi_velocity,
            midi_channel,
            midi_duration
        FROM
            triggers
        WHERE
            idx = :idx
        "#
    )?;

    let result = stmt.query_row(&[(":idx", index.to_string().as_str())], |row| {
        Ok(TriggerRecord {
            index: row.get(0)?,
            position: row.get(1)?,
            enabled: row.get(2)?,
            on_boost: row.get(3)?,
            on_stream: row.get(4)?,
            on_auto: row.get(5)?,
            on_sent: row.get(6)?,
            on_invoice: row.get(7)?,
            amount: row.get(8)?,
            amount_equality: row.get(9)?,
            sender: row.get(10).ok(),
            sender_equality: row.get(11).ok(),
            app: row.get(12).ok(),
            app_equality: row.get(13).ok(),
            podcast: row.get(14).ok(),
            podcast_equality: row.get(15).ok(),
            sound_file: row.get(16).ok(),
            sound_name: row.get(17).ok(),
            webhook_url: row.get(18).ok(),
            webhook_token: row.get(19).ok(),
            webhook_successful: row.get(20).ok(),
            webhook_timestamp: row.get(21).ok(),
            osc_address: row.get(22).ok(),
            osc_port: row.get(23).ok(),
            osc_path: row.get(24).ok(),
            osc_args: row.get(25).ok(),
            osc_successful: row.get(26).ok(),
            osc_timestamp: row.get(27).ok(),
            midi_note: row.get(28).ok(),
            midi_velocity: row.get(29).ok(),
            midi_channel: row.get(30).ok(),
            midi_duration: row.get(31).ok(),
        })
    })?;

    Ok(result)
}

pub fn save_trigger_to_db(filepath: &str, trigger: &TriggerRecord) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let index = if trigger.index > 0 {
        Some(trigger.index)
    } else {
        None
    };

    set_trigger_position_in_db(filepath, trigger.index, trigger.position)?;

    let mut stmt = conn.prepare(
        r#"INSERT INTO triggers (
            idx,
            position,
            enabled,
            on_boost,
            on_stream,
            on_auto,
            on_sent,
            on_invoice,
            amount,
            amount_equality,
            sender,
            sender_equality,
            app,
            app_equality,
            podcast,
            podcast_equality,
            sound_file,
            sound_name,
            webhook_url,
            webhook_token,
            webhook_successful,
            webhook_timestamp,
            osc_address,
            osc_port,
            osc_path,
            osc_args,
            osc_successful,
            osc_timestamp,
            midi_note,
            midi_velocity,
            midi_channel,
            midi_duration
        )
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32)
        ON CONFLICT(idx) DO UPDATE SET
            position = excluded.position,
            enabled = excluded.enabled,
            on_boost = excluded.on_boost,
            on_stream = excluded.on_stream,
            on_auto = excluded.on_auto,
            on_sent = excluded.on_sent,
            on_invoice = excluded.on_invoice,
            amount = excluded.amount,
            amount_equality = excluded.amount_equality,
            sender = excluded.sender,
            sender_equality = excluded.sender_equality,
            app = excluded.app,
            app_equality = excluded.app_equality,
            podcast = excluded.podcast,
            podcast_equality = excluded.podcast_equality,
            sound_file = excluded.sound_file,
            sound_name = excluded.sound_name,
            webhook_url = excluded.webhook_url,
            webhook_token = excluded.webhook_token,
            webhook_successful = excluded.webhook_successful,
            webhook_timestamp = excluded.webhook_timestamp,
            osc_address = excluded.osc_address,
            osc_port = excluded.osc_port,
            osc_path = excluded.osc_path,
            osc_args = excluded.osc_args,
            osc_successful = excluded.osc_successful,
            osc_timestamp = excluded.osc_timestamp,
            midi_note = excluded.midi_note,
            midi_velocity = excluded.midi_velocity,
            midi_channel = excluded.midi_channel,
            midi_duration = excluded.midi_duration
        RETURNING idx
        "#,
    )?;

    let params = params![
        index,
        trigger.position,
        trigger.enabled,
        trigger.on_boost,
        trigger.on_stream,
        trigger.on_auto,
        trigger.on_sent,
        trigger.on_invoice,
        trigger.amount,
        trigger.amount_equality,
        trigger.sender,
        trigger.sender_equality,
        trigger.app,
        trigger.app_equality,
        trigger.podcast,
        trigger.podcast_equality,
        trigger.sound_file,
        trigger.sound_name,
        trigger.webhook_url,
        trigger.webhook_token,
        trigger.webhook_successful,
        trigger.webhook_timestamp,
        trigger.osc_address,
        trigger.osc_port,
        trigger.osc_path,
        trigger.osc_args,
        trigger.osc_successful,
        trigger.osc_timestamp,
        trigger.midi_note,
        trigger.midi_velocity,
        trigger.midi_channel,
        trigger.midi_duration
    ];

    let idx = stmt.query_row(params, |row| {
        let idx: u64 = row.get(0)?;
        Ok(idx)
    })?;

    renumber_triggers_positions_in_db(filepath)?;

    Ok(idx)
}

pub fn set_trigger_position_in_db(filepath: &str, index: u64, position: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    #[allow(clippy::comparison_chain)]
    if index > 0 {
        let current = load_trigger_from_db(filepath, index)?;

        if position < current.position {
            // shift items between the old and new position down by 1
            conn.execute(
                r#"UPDATE triggers SET position = position + 1 WHERE position >= ? AND position <= ? AND idx <> ?"#,
                params![
                    position,
                    current.position,
                    index,
                ]
            )?;
        }
        else if position > current.position {
            // shift items between the old and new position up by 1
            conn.execute(
                r#"UPDATE triggers SET position = position - 1 WHERE position <= ? AND position >= ? AND idx <> ?"#,
                params![
                    position,
                    current.position,
                    index,
                ]
            )?;
        }
    }
    else {
        // shift items down by 1
        conn.execute(
            r#"UPDATE triggers SET position = position + 1 WHERE position >= ?"#,
            params![
                position,
            ]
        )?;
    }


    Ok(true)
}

pub fn renumber_triggers_positions_in_db(filepath: &str) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    // renumber all positions
    conn.execute(
        r#"UPDATE triggers SET position = (SELECT COUNT(*) FROM triggers b WHERE b.position < triggers.position) + 1"#,
        []
    )?;

    Ok(true)
}

pub fn delete_trigger_from_db(filepath: &str, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM triggers WHERE idx = ?1"#, params![index])?;

    renumber_triggers_positions_in_db(filepath)?;

    Ok(true)
}

pub fn set_trigger_webhook_last_request(filepath: &str, index: u64, successful: bool, timestamp: i64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(
        r#"UPDATE triggers SET webhook_successful = ?2, webhook_timestamp = ?3 WHERE idx = ?1"#,
        params![index, successful, timestamp]
    )?;

    Ok(true)
}

pub fn set_trigger_osc_last_request(filepath: &str, index: u64, successful: bool, timestamp: i64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(
        r#"UPDATE triggers SET osc_successful = ?2, osc_timestamp = ?3 WHERE idx = ?1"#,
        params![index, successful, timestamp]
    )?;

    Ok(true)
}