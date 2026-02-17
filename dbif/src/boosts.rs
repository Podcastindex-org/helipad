use rusqlite::{Connection, params};
use std::error::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::HydraError;
use crate::connect_to_database;
use crate::bind_query_param;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BoostRecord {
    pub index: u64,
    pub time: i64,
    pub value_msat: i64,
    pub value_msat_total: i64,
    pub action: u8,
    pub sender: String,
    pub app: String,
    pub message: String,
    pub podcast: String,
    pub episode: String,
    pub tlv: String,
    pub remote_podcast: Option<String>,
    pub remote_episode: Option<String>,
    pub reply_sent: bool,
    pub custom_key: Option<u64>,
    pub custom_value: Option<String>,
    pub payment_info: Option<PaymentRecord>,
}

impl BoostRecord {
    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_html( field: String) -> String {
        field.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    }

    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_csv( field: String) -> String {
        field.replace('"', "\"\"").replace('\n', " ")
    }

    //Parses the TLV record into a Value
    pub fn parse_tlv(&self) -> Result<Value, Box<dyn Error>> {
        Ok(serde_json::from_str(self.tlv.as_str())?)
    }

    // Returns the name of the action
    pub fn action_name(&self) -> String {
        ActionType::from_u8(self.action).as_str().to_string()
    }

    // Returns the name of the action for the list
    pub fn list_type(&self) -> String {
        match self.action_name().as_str() {
            "boost" | "auto" | "invoice" => "boost",
            _ => "stream", // everything else goes into the stream list
        }.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentRecord {
    pub payment_hash: String,
    pub pubkey: String,
    pub custom_key: u64,
    pub custom_value: String,
    pub fee_msat: i64,
    pub reply_to_idx: Option<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct BoostFilters {
    pub podcast: Option<String>,
    pub start_date: Option<u64>,
    pub end_date: Option<u64>,
    pub actions: Vec<ActionType>,
}

impl BoostFilters {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActionType {
    Unknown = 0, // no action type set
    Stream = 1, // streaming payments
    Boost = 2, // manual boost or boost-a-gram
    Invalid = 3, // invalid action or empty string (set to 3 for legacy reasons)
    Auto = 4, // automated boost
    Invoice = 5, // lightning invoice w/message
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::Unknown => "unknown",
            ActionType::Stream => "stream",
            ActionType::Boost => "boost",
            ActionType::Invalid => "invalid",
            ActionType::Auto => "auto",
            ActionType::Invoice => "invoice",
        }
    }

    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => ActionType::Unknown,
            1 => ActionType::Stream,
            2 => ActionType::Boost,
            3 => ActionType::Invalid,
            4 => ActionType::Auto,
            5 => ActionType::Invoice,
            _ => ActionType::Invalid,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "unknown" => ActionType::Unknown,
            "stream" => ActionType::Stream,
            "boost" => ActionType::Boost,
            "invalid" => ActionType::Invalid,
            "auto" => ActionType::Auto,
            "invoice" => ActionType::Invoice,
            _ => ActionType::Invalid,
        }
    }
}


pub fn map_action_to_code(action: &str) -> u8 {
    ActionType::from_str(action) as u8
}

pub fn create_boosts_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    //Create the boosts table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS boosts (
             idx integer primary key,
             time integer,
             value_msat integer,
             value_msat_total integer,
             action integer,
             sender text,
             app text,
             message text,
             podcast text,
             episode text,
             tlv text,
             remote_podcast text,
             remote_episode text,
             custom_key integer,
             custom_value text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Boosts table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to create database boosts table.".into())))
        }
    }

    //Add additional columns to existing installs
    if conn.execute("ALTER TABLE boosts ADD COLUMN remote_podcast text", []).is_ok() {
        println!("Boosts remote podcast column added.");
    }

    if conn.execute("ALTER TABLE boosts ADD COLUMN remote_episode text", []).is_ok() {
        println!("Boosts remote episode column added.");
    }

    if conn.execute("ALTER TABLE boosts ADD COLUMN reply_sent integer", []).is_ok() {
        println!("Boosts reply sent column added.");
    }

    if conn.execute_batch("ALTER TABLE boosts ADD COLUMN custom_key integer; ALTER TABLE boosts ADD COLUMN custom_value text;").is_ok() {
        println!("Boosts custom key/value added.");
    }

    Ok(true)
}


//Add an invoice to the database
pub fn add_invoice_to_db(filepath: &str, boost: &BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute(
        "INSERT INTO boosts
            (idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode, reply_sent, custom_key, custom_value)
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ",
        params![
            boost.index,
            boost.time,
            boost.value_msat,
            boost.value_msat_total,
            boost.action,
            boost.sender,
            boost.app,
            boost.message,
            boost.podcast,
            boost.episode,
            boost.tlv,
            boost.remote_podcast,
            boost.remote_episode,
            boost.reply_sent,
            boost.custom_key,
            boost.custom_value
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(Box::new(HydraError(format!("Failed to add boost: [{}].", boost.index))))
        }
    }
}

//Set the boost as replied to
pub fn mark_boost_as_replied(filepath: &str, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    conn.execute("UPDATE boosts SET reply_sent = 1 WHERE idx = ?1", params![index])?;
    Ok(true)
}

//Update an existing invoice with new data (e.g., from payment metadata fetch)
pub fn update_invoice_in_db(filepath: &str, boost: &BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute(
        "UPDATE boosts SET
            value_msat = ?1,
            value_msat_total = ?2,
            action = ?3,
            sender = ?4,
            app = ?5,
            message = ?6,
            podcast = ?7,
            episode = ?8,
            tlv = ?9,
            remote_podcast = ?10,
            remote_episode = ?11,
            custom_key = ?12,
            custom_value = ?13
        WHERE idx = ?14
        ",
        params![
            boost.value_msat,
            boost.value_msat_total,
            boost.action,
            boost.sender,
            boost.app,
            boost.message,
            boost.podcast,
            boost.episode,
            boost.tlv,
            boost.remote_podcast,
            boost.remote_episode,
            boost.custom_key,
            boost.custom_value,
            boost.index
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(Box::new(HydraError(format!("Failed to update boost: [{}].", boost.index))))
        }
    }
}

//Get all of the invoices from the database
pub fn get_invoices_from_db(filepath: &str, invtype: &str, index: u64, max: u64, direction: bool, escape_html: bool, filters: BoostFilters) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut conditions: Vec<&str> = Vec::new();
    let mut bindings: HashMap<&str, &str> = HashMap::new();

    let cond = if direction {
        "idx <= :idx"
    } else {
        "idx >= :idx"
    };

    conditions.push(cond);

    let strindex = index.to_string();
    bindings.insert(":idx", &strindex);

    if invtype == "boost" {
        conditions.push("action IN (2, 4, 5)");
    }
    else if invtype == "stream" {
        conditions.push("action NOT IN (2, 4, 5)");
    }

    let mut action_filters= HashMap::new();
    let action_condition_string;

    if !filters.actions.is_empty() {
        for (idx, action) in filters.actions.iter().enumerate() {
            let key = format!(":action{}", idx);
            let value = (*action as u8).to_string();
            action_filters.insert(key, value);
        }

        let action_condition_list = action_filters
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<&str>>()
            .join(", ");

        action_condition_string = format!("action IN ({})", action_condition_list);
        conditions.push(action_condition_string.as_str());

        for (key, value) in &action_filters {
            bindings.insert(key.as_str(), value.as_str());
        }
    }

    if let Some(podcast) = &filters.podcast {
        conditions.push("podcast = :podcast");
        bindings.insert(":podcast", podcast);
    }

    let start_date = filters.start_date.unwrap_or_default().to_string();

    if !start_date.is_empty() && start_date != "0" {
        conditions.push("time >= :start_date");
        bindings.insert(":start_date", &start_date);
    }

    let end_date = filters.end_date.unwrap_or_default().to_string();

    if !end_date.is_empty() && end_date != "0" {
        conditions.push("time <= :end_date");
        bindings.insert(":end_date", &end_date);
    }

    let conditions = conditions.join(" AND ");

    let mut limit = String::new();
    let strmax = max.to_string();

    if max > 0 {
        limit.push_str("LIMIT :max");
        bindings.insert(":max", &strmax);
    }

    //Query for boosts and automated boosts
    let sqltxt = format!(
        "SELECT
            idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode, reply_sent, custom_key, custom_value
        FROM
            boosts
        WHERE
            {}
        ORDER BY
            idx DESC
        {}
        ",
        conditions,
        limit
    );

    //Prepare and execute the query
    let mut stmt = conn.prepare(sqltxt.as_str())?;

    for (name, value) in &bindings {
        bind_query_param(&mut stmt, name, value)?;
    }

    let mut rows = stmt.raw_query();
    let mut boosts: Vec<BoostRecord> = Vec::new();

    while let Some(row) = rows.next()? {
        let boost = BoostRecord {
            index: row.get(0)?,
            time: row.get(1)?,
            value_msat: row.get(2)?,
            value_msat_total: row.get(3)?,
            action: row.get(4)?,
            sender: row.get(5)?,
            app: row.get(6)?,
            message: row.get(7)?,
            podcast: row.get(8)?,
            episode: row.get(9)?,
            tlv: row.get(10)?,
            remote_podcast: row.get(11).ok(),
            remote_episode: row.get(12).ok(),
            reply_sent: row.get(13).unwrap_or(false),
            custom_key: row.get(14).ok(),
            custom_value: row.get(15).ok(),
            payment_info: None,
        };

        //Some things like text output don't need to be html entity escaped
        //so only do it if asked for
        if escape_html {
            let boost_clean = BoostRecord {
                sender: BoostRecord::escape_for_html(boost.sender),
                app: BoostRecord::escape_for_html(boost.app),
                message: BoostRecord::escape_for_html(boost.message),
                podcast: BoostRecord::escape_for_html(boost.podcast),
                episode: BoostRecord::escape_for_html(boost.episode),
                tlv: BoostRecord::escape_for_html(boost.tlv),
                remote_podcast: boost.remote_podcast.map(BoostRecord::escape_for_html),
                remote_episode: boost.remote_episode.map(BoostRecord::escape_for_html),
                ..boost
            };
            boosts.push(boost_clean);
        } else {
            boosts.push(boost);
        }

    }

    Ok(boosts)
}

pub fn get_single_invoice_from_db(filepath: &str, index: u64, escape_html: bool) -> Result<Option<BoostRecord>, Box<dyn Error>> {
    let filters = BoostFilters::new();
    let invoices = get_invoices_from_db(filepath, "", index, 1, true, escape_html, filters)?;

    if !invoices.is_empty() && invoices[0].index == index {
        Ok(Some(invoices[0].clone()))
    }
    else {
        Ok(None)
    }
}

pub fn get_boosts_from_db(filepath: &str, index: u64, max: u64, direction: bool, escape_html: bool, filters: BoostFilters) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    get_invoices_from_db(filepath, "boost", index, max, direction, escape_html, filters)
}

//Get all of the non-boosts from the database
pub fn get_streams_from_db(filepath: &str, index: u64, max: u64, direction: bool, escape_html: bool, filters: BoostFilters) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    get_invoices_from_db(filepath, "stream", index, max, direction, escape_html, filters)
}

//Get the last boost index number from the database
pub fn get_last_boost_index_from_db(filepath: &str) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();
    let max = 1;

    //Prepare and execute the query
    let mut stmt = conn.prepare(
        "SELECT
            idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode, reply_sent, custom_key, custom_value
        FROM
            boosts
        ORDER BY
            idx DESC
        LIMIT
            :max
        "
    )?;

    let rows = stmt.query_map(&[(":max", max.to_string().as_str())], |row| {
        Ok(BoostRecord {
            index: row.get(0)?,
            time: row.get(1)?,
            value_msat: row.get(2)?,
            value_msat_total: row.get(3)?,
            action: row.get(4)?,
            sender: row.get(5)?,
            app: row.get(6)?,
            message: row.get(7)?,
            podcast: row.get(8)?,
            episode: row.get(9)?,
            tlv: row.get(10)?,
            remote_podcast: row.get(11).ok(),
            remote_episode: row.get(12).ok(),
            reply_sent: row.get(13).unwrap_or(false),
            custom_key: row.get(14).ok(),
            custom_value: row.get(15).ok(),
            payment_info: None,
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let boost: BoostRecord = row.unwrap();
        boosts.push(boost);
    }

    if !boosts.is_empty() {
        return Ok(boosts[0].index)
    }

    Ok(0)
}

//Get podcasts that received boosts to this node
pub fn get_podcasts_from_db(filepath: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let query = "SELECT DISTINCT podcast FROM boosts WHERE podcast <> '' ORDER BY podcast".to_string();

    let mut stmt = conn.prepare(&query)?;
    let mut rows = stmt.raw_query();

    //Parse the results
    let mut podcasts = Vec::new();

    while let Some(row) = rows.next()? {
        podcasts.push(row.get(0)?);
    }

    Ok(podcasts)
}
