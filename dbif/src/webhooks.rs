use rusqlite::{Connection, params};
use std::error::Error;
use serde::{Deserialize, Serialize};
use chrono::DateTime;
use crate::{connect_to_database, HydraError};
#[derive(Serialize, Deserialize, Debug, Clone)]

pub struct WebhookRecord {
    pub index: u64,
    pub url: String,
    pub token: String,
    pub on_boost: bool,
    pub on_stream: bool,
    pub on_auto: bool,
    pub on_sent: bool,
    pub on_invoice: bool,
    pub equality: String,
    pub amount: u64,
    pub enabled: bool,
    pub request_successful: Option<bool>,
    pub request_timestamp: Option<i64>,
}

impl WebhookRecord {
    pub fn get_request_timestamp_string(&self) -> Option<String> {
        match self.request_timestamp {
            Some(timestamp) => DateTime::from_timestamp(timestamp, 0).map(|ts| ts.to_rfc3339()),
            None => None,
        }
    }
}

pub fn create_webhooks_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    //Create the webhooks table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS webhooks (
             idx integer primary key autoincrement,
             url text,
             token text,
             on_boost integer,
             on_stream integer,
             on_sent integer,
             enabled integer,
             request_successful integer,
             request_timestamp integer
         )",
        [],
    ) {
        Ok(_) => {
            println!("Webhooks table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to create database webhooks table.".into())))
        }
    }

    if conn.execute_batch(
        "ALTER TABLE webhooks ADD COLUMN equality text DEFAULT '';
        ALTER TABLE webhooks ADD COLUMN amount integer DEFAULT 0;"
    ).is_ok() {
        println!("Webhook amounts added");
    }

    if conn.execute("ALTER TABLE webhooks ADD COLUMN on_auto integer DEFAULT 0", []).is_ok() {
        // Set on_auto to 1 if on_boost is 1 for backward compatibility
        if conn.execute("UPDATE webhooks SET on_auto = 1 WHERE on_boost = 1", []).is_ok() {
            println!("Webhook on_auto field added and migrated from on_boost");
        }
    }

    if conn.execute("ALTER TABLE webhooks ADD COLUMN on_invoice integer DEFAULT 0", []).is_ok() {
        println!("Webhook on_invoice field added");
    }

    Ok(true)
}

pub fn get_webhooks_from_db(filepath: &str, enabled: Option<bool>) -> Result<Vec<WebhookRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut webhooks: Vec<WebhookRecord> = Vec::new();

    let where_enabled = match enabled {
        Some(true) => "WHERE enabled = 1",
        Some(false) => "WHERE enabled = 0",
        None => "",
    };

    let sqltxt = format!(
        r#"SELECT
            idx,
            url,
            token,
            on_boost,
            on_stream,
            on_auto,
            on_sent,
            on_invoice,
            equality,
            amount,
            enabled,
            request_successful,
            request_timestamp
        FROM
            webhooks
        {}"#,
        where_enabled,
    );

    let mut stmt = conn.prepare(sqltxt.as_str())?;
    let rows = stmt.query_map([], |row| {
        Ok(WebhookRecord {
            index: row.get(0)?,
            url: row.get(1)?,
            token: row.get(2)?,
            on_boost: row.get(3)?,
            on_stream: row.get(4)?,
            on_auto: row.get(5)?,
            on_sent: row.get(6)?,
            on_invoice: row.get(7)?,
            equality: row.get(8)?,
            amount: row.get(9)?,
            enabled: row.get(10)?,
            request_successful: row.get(11).ok(),
            request_timestamp: row.get(12).ok(),
        })
    }).unwrap();

    for row in rows {
        webhooks.push(row.unwrap());
    }

    Ok(webhooks)
}

pub fn load_webhook_from_db(filepath: &str, index: u64) -> Result<WebhookRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            url,
            token,
            on_boost,
            on_stream,
            on_auto,
            on_sent,
            on_invoice,
            equality,
            amount,
            enabled,
            request_successful,
            request_timestamp
        FROM
            webhooks
        WHERE
            idx = :idx
        "#
    )?;

    let webhook = stmt.query_row(&[(":idx", index.to_string().as_str())], |row| {
        Ok(WebhookRecord {
            index: row.get(0)?,
            url: row.get(1)?,
            token: row.get(2)?,
            on_boost: row.get(3)?,
            on_stream: row.get(4)?,
            on_auto: row.get(5)?,
            on_sent: row.get(6)?,
            on_invoice: row.get(7)?,
            equality: row.get(8)?,
            amount: row.get(9)?,
            enabled: row.get(10)?,
            request_successful: row.get(11).ok(),
            request_timestamp: row.get(12).ok(),
        })
    })?;

    Ok(webhook)
}

pub fn save_webhook_to_db(filepath: &str, webhook: &WebhookRecord) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let index = if webhook.index > 0 {
        Some(webhook.index)
    } else {
        None
    };

    let mut stmt = conn.prepare(
        r#"INSERT INTO webhooks (
            idx,
            url,
            token,
            on_boost,
            on_stream,
            on_auto,
            on_sent,
            on_invoice,
            equality,
            amount,
            enabled,
            request_successful,
            request_timestamp
        )
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(idx) DO UPDATE SET
            url = excluded.url,
            token = excluded.token,
            on_boost = excluded.on_boost,
            on_stream = excluded.on_stream,
            on_auto = excluded.on_auto,
            on_sent = excluded.on_sent,
            on_invoice = excluded.on_invoice,
            equality = excluded.equality,
            amount = excluded.amount,
            enabled = excluded.enabled
        RETURNING idx
        "#,
    )?;

    let params = params![
        index,
        webhook.url,
        webhook.token,
        webhook.on_boost,
        webhook.on_stream,
        webhook.on_auto,
        webhook.on_sent,
        webhook.on_invoice,
        webhook.equality,
        webhook.amount,
        webhook.enabled,
        webhook.request_successful,
        webhook.request_timestamp,
    ];

    let idx = stmt.query_row(params, |row| {
        let idx: u64 = row.get(0)?;
        Ok(idx)
    })?;

    Ok(idx)
}

pub fn set_webhook_last_request(filepath: &str, index: u64, successful: bool, timestamp: i64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(
        r#"UPDATE webhooks SET request_successful = ?2, request_timestamp = ?3 WHERE idx = ?1"#,
        params![index, successful, timestamp]
    )?;

    Ok(true)

}

pub fn delete_webhook_from_db(filepath: &str, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM webhooks WHERE idx = ?1"#, params![index])?;

    Ok(true)
}
