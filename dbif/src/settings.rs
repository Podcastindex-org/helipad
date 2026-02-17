use rusqlite::{Connection, params, Error::QueryReturnedNoRows};
use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::{connect_to_database, HydraError};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsRecord {
    pub show_received_sats: bool,
    pub show_split_percentage: bool,
    pub hide_boosts: bool,
    pub hide_boosts_below: Option<u64>,
    pub play_pew: bool,
    pub custom_pew_file: Option<String>,
    pub resolve_nostr_refs: bool,
    pub show_hosted_wallet_ids: bool,
    pub show_lightning_invoices: bool,
    pub fetch_metadata: bool,
}

pub fn create_settings_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    //Create the settings table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
             idx integer primary key autoincrement,
             show_received_sats integer not null,
             show_split_percentage integer not null,
             hide_boosts integer not null,
             hide_boosts_below integer,
             play_pew integer not null,
             custom_pew_file text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Settings table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to create database settings table.".into())))
        }
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN resolve_nostr_refs integer DEFAULT 0", []).is_ok() {
        println!("Nostr refs setting added.");
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN show_hosted_wallet_ids integer DEFAULT 0", []).is_ok() {
        println!("Hosted wallet id setting added.");
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN show_lightning_invoices integer DEFAULT 1", []).is_ok() {
        println!("Show lightning invoices setting added.");
    }

    if conn.execute("ALTER TABLE settings ADD COLUMN fetch_metadata integer DEFAULT 1", []).is_ok() {
        println!("Fetch metadata setting added.");
    }

    Ok(true)
}

pub fn load_settings_from_db(filepath: &str) -> Result<SettingsRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
             show_received_sats,
             show_split_percentage,
             hide_boosts,
             hide_boosts_below,
             play_pew,
             custom_pew_file,
             resolve_nostr_refs,
             show_hosted_wallet_ids,
             show_lightning_invoices,
             fetch_metadata
        FROM
            settings
        WHERE
            idx = 1
        "#
    )?;

    let result = stmt.query_row([], |row| {
        Ok(SettingsRecord {
            show_received_sats: row.get(0)?,
            show_split_percentage: row.get(1)?,
            hide_boosts: row.get(2)?,
            hide_boosts_below: row.get(3).ok(),
            play_pew: row.get(4)?,
            custom_pew_file: row.get(5).ok(),
            resolve_nostr_refs: row.get(6)?,
            show_hosted_wallet_ids: row.get(7)?,
            show_lightning_invoices: row.get(8)?,
            fetch_metadata: row.get(9).unwrap_or(true),
        })
    });

    match result {
        Ok(s) => Ok(s),
        Err(QueryReturnedNoRows) => Ok(SettingsRecord {
            show_received_sats: false,
            show_split_percentage: false,
            hide_boosts: false,
            hide_boosts_below: None,
            play_pew: true,
            custom_pew_file: None,
            resolve_nostr_refs: false,
            show_hosted_wallet_ids: false,
            show_lightning_invoices: true,
            fetch_metadata: true,
        }),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn save_settings_to_db(filepath: &str, settings: &SettingsRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute(
        r#"INSERT INTO settings (
            idx,
            show_received_sats,
            show_split_percentage,
            hide_boosts,
            hide_boosts_below,
            play_pew,
            custom_pew_file,
            resolve_nostr_refs,
            show_hosted_wallet_ids,
            show_lightning_invoices,
            fetch_metadata
        )
        VALUES
            (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(idx) DO UPDATE SET
            show_received_sats = excluded.show_received_sats,
            show_split_percentage = excluded.show_split_percentage,
            hide_boosts = excluded.hide_boosts,
            hide_boosts_below = excluded.hide_boosts_below,
            play_pew = excluded.play_pew,
            custom_pew_file = excluded.custom_pew_file,
            resolve_nostr_refs = excluded.resolve_nostr_refs,
            show_hosted_wallet_ids = excluded.show_hosted_wallet_ids,
            show_lightning_invoices = excluded.show_lightning_invoices,
            fetch_metadata = excluded.fetch_metadata
        "#,
        params![
            settings.show_received_sats,
            settings.show_split_percentage,
            settings.hide_boosts,
            settings.hide_boosts_below,
            settings.play_pew,
            settings.custom_pew_file,
            settings.resolve_nostr_refs,
            settings.show_hosted_wallet_ids,
            settings.show_lightning_invoices,
            settings.fetch_metadata,
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(Box::new(HydraError("Failed to save settings".into())))
        }
    }
}