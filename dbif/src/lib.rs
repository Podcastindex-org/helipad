use rusqlite::{params, Connection, Error::QueryReturnedNoRows};
use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;
use chrono::DateTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfoRecord {
    pub lnd_alias: String,
    pub node_pubkey: String,
    pub node_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
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
    pub payment_info: Option<PaymentRecord>,
}

impl BoostRecord {
    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_html( field: String) -> String {
        return field.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
    }

    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_csv( field: String) -> String {
        return field.replace("\"", "\"\"").replace("\n", " ");
    }

    //Parses the TLV record into a Value
    pub fn parse_tlv(&self) -> Result<Value, Box<dyn Error>> {
        return Ok(serde_json::from_str(self.tlv.as_str())?);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaymentRecord {
    pub payment_hash: String,
    pub pubkey: String,
    pub custom_key: u64,
    pub custom_value: String,
    pub fee_msat: i64,
    pub reply_to_idx: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NumerologyRecord {
    pub index: u64,
    pub amount: u64,
    pub equality: String,
    pub emoji: Option<String>,
    pub sound_file: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebhookRecord {
    pub index: u64,
    pub url: String,
    pub token: String,
    pub on_boost: bool,
    pub on_stream: bool,
    pub on_sent: bool,
    pub enabled: bool,
    pub request_successful: Option<bool>,
    pub request_timestamp: Option<i64>,
}

impl WebhookRecord {
    pub fn get_request_timestamp_string(&self) -> Option<String> {
        match self.request_timestamp {
            Some(timestamp) => match DateTime::from_timestamp(timestamp, 0) {
                Some(ts) => Some(ts.to_rfc3339()),
                None => None,
            },
            None => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingsRecord {
    pub show_received_sats: bool,
    pub show_split_percentage: bool,
    pub hide_boosts: bool,
    pub hide_boosts_below: Option<u64>,
    pub play_pew: bool,
    pub custom_pew_file: Option<String>,
}

#[derive(Debug)]
struct HydraError(String);
impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}
impl Error for HydraError {}


//Connect to the database at the given file location
fn connect_to_database(init: bool, filepath: &String) -> Result<Connection, Box<dyn Error>> {
    if let Ok(conn) = Connection::open(filepath.as_str()) {
        if init {
            match set_database_file_permissions(filepath.as_str()) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("{:#?}", e);
                }
            }
            println!("Using database file: [{}]", filepath.as_str());
        }
        Ok(conn)
    } else {
        return Err(Box::new(HydraError(format!("Could not open a database file at: [{}].", filepath).into())))
    }
}

//Set permissions on the database file
fn set_database_file_permissions(filepath: &str) -> Result<bool, Box<dyn Error>> {

    match std::fs::File::open(filepath) {
        Ok(fh) => {
            match fh.metadata() {
                Ok(metadata) => {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o666);
                    println!("Set file permission to: [666] on database file: [{}]", filepath);
                    Ok(true)
                },
                Err(e) => {
                    return Err(Box::new(HydraError(format!("Error getting metadata from database file handle: [{}].  Error: {:#?}.", filepath, e).into())))
                }
            }
        },
        Err(e) => {
            return Err(Box::new(HydraError(format!("Error opening database file handle: [{}] for permissions setting.  Error: {:#?}.", filepath, e).into())))
        }
    }
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, Box<dyn Error>> {
    //Prepare and execute the query
    let mut stmt = conn.prepare(r#"SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1"#)?;
    let rows = stmt.query_map(params![table_name], |_| Ok(true))?;

    for _ in rows {
        return Ok(true);
    }

    Ok(false)
}

//Create or update a new database file if needed
pub fn create_database(filepath: &String) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(true, filepath)?;

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
             remote_episode text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Boosts table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to create database boosts table: [{}].", filepath).into())))
        }
    }

    //Add additional columns to existing installs
    match conn.execute("ALTER TABLE boosts ADD COLUMN remote_podcast text", []) {
        Ok(_) => {
            println!("Boosts remote podcast column added.");
        }
        Err(_) => {}
    }

    match conn.execute("ALTER TABLE boosts ADD COLUMN remote_episode text", []) {
        Ok(_) => {
            println!("Boosts remote episode column added.");
        }
        Err(_) => {}
    }

    match conn.execute("ALTER TABLE boosts ADD COLUMN reply_sent integer", []) {
        Ok(_) => {
            println!("Boosts reply sent column added.");
        }
        Err(_) => {}
    }

    //Create the node info table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS node_info (
             idx integer primary key,
             time integer,
             lnd_info text,
             last_connection_status integer,
             last_connection_status_message text,
             alert_message text,
             wallet_balance integer,
             chain_balance integer,
             block_height integer,
             current_lnd_index integer,
             liquidity_danger integer,
             chain_sync_status integer,
             graph_sync_status integer,
             lnd_alias text,
             node_pubkey text,
             node_version text,
             info_int_1 integer,
             info_int_2 integer,
             info_int_3 integer,
             info_int_4 integer,
             info_int_5 integer,
             info_int_6 integer,
             info_int_7 integer,
             info_int_8 integer,
             info_int_9 integer,
             info_int_10 integer,
             info_text_1 text,
             info_text_2 text,
             info_text_3 text,
             info_text_4 text,
             info_text_5 text,
             info_text_6 text,
             info_text_7 text,
             info_text_8 text,
             info_text_9 text,
             info_text_10 text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Node info table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to create database node_info table: [{}].", filepath).into())))
        }
    }

    //Create the sent boosts table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS sent_boosts (
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
             payment_hash text,
             payment_pubkey text,
             payment_custom_key integer,
             payment_custom_value text,
             payment_fee_msat integer,
             reply_to_idx integer
         )",
        [],
    ) {
        Ok(_) => {
            println!("Sent boosts table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to create database sent_boosts table: [{}].", filepath).into())))
        }
    }

    //Create the numerology table
    let numerology_exists = table_exists(&conn, "numerology")?;

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS numerology (
             idx integer primary key,
             equality text not null,
             amount integer not null,
             emoji text,
             sound_file text,
             description text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Numerology table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to create database numerology table: [{}].", filepath).into())))
        }
    }

    if !numerology_exists {
        if insert_default_numerology(&conn)? {
            println!("Default numerology added.");
        }
    }

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
            return Err(Box::new(HydraError(format!("Failed to create database settings table: [{}].", filepath).into())))
        }
    }

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
            return Err(Box::new(HydraError(format!("Failed to create database webhooks table: [{}].", filepath).into())))
        }
    }

    Ok(true)
}

pub fn insert_default_numerology(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    let queries = vec![
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Satchel of Richards Donation x 7', '🍆🍆🍆🍆🍆🍆🍆', '1111111', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Satchel of Richards Donation x 6', '🍆🍆🍆🍆🍆🍆', '111111', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Satchel of Richards Donation x 5', '🍆🍆🍆🍆🍆', '11111', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Satchel of Richards Donation x 4', '🍆🍆🍆🍆', '1111', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Satchel of Richards Donation x 3', '🍆🍆🍆', '111', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Satchel of Richards Donation x 2', '🍆🍆', '11', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Ducks In a Row Donation x 7', '🦆🦆🦆🦆🦆🦆🦆', '2222222', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Ducks In a Row Donation x 6', '🦆🦆🦆🦆🦆🦆', '222222', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Ducks In a Row Donation x 5', '🦆🦆🦆🦆🦆', '22222', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Ducks In a Row Donation x 4', '🦆🦆🦆🦆', '2222', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Ducks In a Row Donation x 3', '🦆🦆🦆', '222', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Ducks In a Row Donation x 2', '🦆🦆', '22', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swan Donation x 7', '🦢🦢🦢🦢🦢🦢🦢', '5555555', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swan Donation x 6', '🦢🦢🦢🦢🦢🦢', '555555', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swan Donation x 5', '🦢🦢🦢🦢🦢', '55555', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swan Donation x 4', '🦢🦢🦢🦢', '5555', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swan Donation x 3', '🦢🦢🦢', '555', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swan Donation x 2', '🦢🦢', '55', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countdown Donation x 5', '💥💥💥💥💥', '7654321', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countdown Donation x 4', '💥💥💥💥', '654321', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countdown Donation x 3', '💥💥💥', '54321', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countdown Donation x 2', '💥💥', '4321', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countdown Donation', '💥', '321', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countup Donation x 5', '🧛🧛🧛🧛🧛', '1234567', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countup Donation x 4', '🧛🧛🧛🧛', '123456', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countup Donation x 3', '🧛🧛🧛', '12345', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countup Donation x 2', '🧛🧛', '1234', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Countup Donation', '🧛', '123', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Bowler Donation x 3 +🦃', '🎳🎳🎳🦃', '101010', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Bowler Donation x 2', '🎳🎳', '1010', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Bowler Donation', '🎳', '10', '=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Dice Donation', '🎲', '11', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Bitcoin donation', '🪙', '21', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Magic Number Donation', '✨', '33', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Swasslenuff Donation', '💋', '69', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Greetings Donation', '👋', '73', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Love and Kisses Donation', '🥰', '88', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Stoner Donation', '✌👽💨', '420', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Devil Donation', '😈', '666', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Angel Donation', '😇', '777', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('America Fuck Yeah Donation', '🇺🇸', '1776', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Canada Donation', '🇨🇦', '1867', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Boobs Donation', '🎱🎱', '6006', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Boobs Donation', '🎱🎱', '8008', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Wolf Donation', '🐺', '9653', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Boost Donation', '🔁', '30057', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Pi Donation x 5', '🥧🥧🥧🥧🥧', '3141592', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Pi Donation x 4', '🥧🥧🥧🥧', '314159', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Pi Donation x 3', '🥧🥧🥧', '31415', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Pi Donation x 2', '🥧🥧', '3141', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Pi Donation', '🥧', '314', '=~')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Poo donation', '💩', '9', '<')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Lit donation 100k', '🔥', '100000', '>=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Lit donation 50k', '🔥', '50000', '>=')",
        "INSERT INTO numerology (description, emoji, amount, equality) VALUES ('Lit donation 10k', '🔥', '10000', '>=')",
    ];

    for query in queries {
        let result = conn.execute(query, []);

        if let Err(e) = result {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to insert default numerology".into())))
        }
    }

    Ok(true)
}

pub fn get_node_info_from_db(filepath: &String) -> Result<NodeInfoRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    //Prepare and execute the query
    let mut stmt = conn.prepare("
        SELECT
            lnd_alias,
            node_pubkey,
            node_version
        FROM
            node_info
        WHERE
            idx = 1
    ")?;

    let rows = stmt.query_map([], |row| {
        Ok(NodeInfoRecord {
            lnd_alias: row.get(0)?,
            node_pubkey: row.get(1)?,
            node_version: row.get(2)?,
        })
    })?;

    // Return first record if found
    for row in rows {
        return Ok(row?);
    }

    // else return empty record
    Ok(NodeInfoRecord {
        lnd_alias: "".into(),
        node_pubkey: "".into(),
        node_version: "".into(),
    })
}

//Add an invoice to the database
pub fn add_node_info_to_db(filepath: &String, info: NodeInfoRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute("
        INSERT INTO node_info
            (idx, lnd_alias, node_pubkey, node_version)
        VALUES
            (1, ?1, ?2, ?3)
        ON CONFLICT(idx) DO UPDATE SET
            lnd_alias = excluded.lnd_alias,
            node_pubkey = excluded.node_pubkey,
            node_version = excluded.node_version
        ",
        params![
            info.lnd_alias,
            info.node_pubkey,
            info.node_version,
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to add node info".into())))
        }
    }
}

//Add an invoice to the database
pub fn add_invoice_to_db(filepath: &String, boost: &BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute("INSERT INTO boosts (idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode, reply_sent) \
                                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                       params![boost.index,
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
                                       boost.reply_sent]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to add boost: [{}].", boost.index).into())))
        }
    }
}

//Set the boost as replied to
pub fn mark_boost_as_replied(filepath: &String, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    conn.execute("UPDATE boosts SET reply_sent = 1 WHERE idx = ?1", params![index])?;
    Ok(true)
}

//Get all of the boosts from the database
pub fn get_boosts_from_db(filepath: &String, index: u64, max: u64, direction: bool, escape_html: bool) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();

    let mut ltgt = ">=";
    if direction {
        ltgt = "<=";
    }

    //Query for boosts and automated boosts
    let sqltxt = format!("SELECT idx, \
                                       time, \
                                       value_msat, \
                                       value_msat_total, \
                                       action, \
                                       sender, \
                                       app, \
                                       message, \
                                       podcast, \
                                       episode, \
                                       tlv, \
                                       remote_podcast, \
                                       remote_episode, \
                                       reply_sent \
                                 FROM boosts \
                                 WHERE action IN (2, 4) \
                                   AND idx {} :index \
                                 ORDER BY idx DESC \
                                 LIMIT :max", ltgt);

    //Prepare and execute the query
    let mut stmt = conn.prepare(sqltxt.as_str())?;
    let rows = stmt.query_map(&[(":index", index.to_string().as_str()), (":max", max.to_string().as_str())], |row| {
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
            payment_info: None,
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let boost: BoostRecord = row.unwrap();

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
                remote_podcast: match boost.remote_podcast {
                    Some(item) => Some(BoostRecord::escape_for_html(item)),
                    None => None
                },
                remote_episode: match boost.remote_episode {
                    Some(item) => Some(BoostRecord::escape_for_html(item)),
                    None => None
                },
                ..boost
            };
            boosts.push(boost_clean);
        } else {
            boosts.push(boost);
        }

    }

    Ok(boosts)
}


//Get all of the non-boosts from the database
pub fn get_streams_from_db(filepath: &String, index: u64, max: u64, direction: bool, escape_html: bool) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();


    let mut ltgt = ">=";
    if direction {
        ltgt = "<=";
    }

    //Build the query to include anything that's not a boost or auto boost
    let sqltxt = format!("SELECT idx, \
                                       time, \
                                       value_msat, \
                                       value_msat_total, \
                                       action, \
                                       sender, \
                                       app, \
                                       message, \
                                       podcast, \
                                       episode, \
                                       tlv, \
                                       remote_podcast, \
                                       remote_episode, \
                                       reply_sent \
                                 FROM boosts \
                                 WHERE action NOT IN (2, 4) \
                                   AND idx {} :index \
                                 ORDER BY idx DESC \
                                 LIMIT :max", ltgt);

    //Prepare and execute the query
    let mut stmt = conn.prepare(sqltxt.as_str())?;
    let rows = stmt.query_map(&[(":index", index.to_string().as_str()), (":max", max.to_string().as_str())], |row| {
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
            payment_info: None,
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let boost: BoostRecord = row.unwrap();

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
                remote_podcast: match boost.remote_podcast {
                    Some(item) => Some(BoostRecord::escape_for_html(item)),
                    None => None
                },
                remote_episode: match boost.remote_episode {
                    Some(item) => Some(BoostRecord::escape_for_html(item)),
                    None => None
                },
                ..boost
            };
            boosts.push(boost_clean);
        } else {
            boosts.push(boost);
        }

    }

    Ok(boosts)
}

//Get the last boost index number from the database
pub fn get_last_boost_index_from_db(filepath: &String) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();
    let max = 1;

    //Prepare and execute the query
    let mut stmt = conn.prepare("SELECT idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode, reply_sent \
                                 FROM boosts \
                                 ORDER BY idx DESC LIMIT :max")?;
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
            payment_info: None,
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let boost: BoostRecord = row.unwrap();
        boosts.push(boost);
    }

    if boosts.len() > 0 {
        return Ok(boosts[0].index)
    }

    Ok(0)
}

//Set/Get the wallet balance from the database in sats
pub fn add_wallet_balance_to_db(filepath: &String, balance: i64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute("INSERT INTO node_info (idx, wallet_balance) \
                                  VALUES (1, ?1) \
                                  ON CONFLICT(idx) DO UPDATE SET wallet_balance = ?1",
                       params![balance]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to update wallet balance in database: [{}].", balance).into())))
        }
    }
}
pub fn get_wallet_balance_from_db(filepath: &String) -> Result<i64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    //Prepare and execute the query
    let mut stmt = conn.prepare("SELECT wallet_balance \
                                               FROM node_info \
                                               WHERE idx = 1")?;
    let rows = stmt.query_map([], |row| row.get(0))?;

    let mut info = Vec::new();

    for info_result in rows {
        info.push(info_result?);
    }

    Ok(info[0])
}

//Get all of the sent boosts from the database
pub fn get_payments_from_db(filepath: &String, index: u64, max: u64, direction: bool, escape_html: bool) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();

    let mut ltgt = ">=";
    if direction {
        ltgt = "<=";
    }

    //Build the query
    let sqltxt = format!(
        "SELECT
            idx,
            time,
            value_msat,
            value_msat_total,
            action,
            sender,
            app,
            message,
            podcast,
            episode,
            tlv,
            remote_podcast,
            remote_episode,
            payment_hash,
            payment_pubkey,
            payment_custom_key,
            payment_custom_value,
            payment_fee_msat,
            reply_to_idx
        FROM
            sent_boosts
        WHERE
            idx {} :index
        ORDER BY
            idx DESC
        LIMIT
            :max
        ",
        ltgt
    );

    //Prepare and execute the query
    let mut stmt = conn.prepare(sqltxt.as_str())?;
    let rows = stmt.query_map(&[(":index", index.to_string().as_str()), (":max", max.to_string().as_str())], |row| {
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
            reply_sent: false,
            payment_info: Some(PaymentRecord {
                payment_hash: row.get(13)?,
                pubkey: row.get(14)?,
                custom_key: row.get(15)?,
                custom_value: row.get(16)?,
                fee_msat: row.get(17)?,
                reply_to_idx: row.get(18)?,
            }),
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let boost: BoostRecord = row.unwrap();

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
                remote_podcast: match boost.remote_podcast {
                    Some(item) => Some(BoostRecord::escape_for_html(item)),
                    None => None
                },
                remote_episode: match boost.remote_episode {
                    Some(item) => Some(BoostRecord::escape_for_html(item)),
                    None => None
                },
                payment_info: match boost.payment_info {
                    Some(info) => Some(PaymentRecord {
                        pubkey: BoostRecord::escape_for_html(info.pubkey),
                        custom_value: BoostRecord::escape_for_html(info.custom_value),
                        ..info
                    }),
                    None => None,
                },
                ..boost
            };
            boosts.push(boost_clean);
        } else {
            boosts.push(boost);
        }

    }

    Ok(boosts)
}

pub fn get_last_payment_index_from_db(filepath: &String) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare("SELECT MAX(idx) FROM sent_boosts")?;
    let index = stmt.query_row([], |row| row.get(0))?;

    if let Some(idx) = index {
        return Ok(idx);
    }

    Ok(0)
}

//Add a payment (sent boost) to the database
pub fn add_payment_to_db(filepath: &String, boost: &BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let payment_info = match &boost.payment_info {
        Some(info) => info,
        None => {
            return Err(Box::new(HydraError(format!("Missing payment info for sent boost: [{}].", boost.index).into())))
        }
    };

    conn.execute(
        "INSERT INTO sent_boosts (
            idx,
            time,
            value_msat,
            value_msat_total,
            action,
            sender,
            app,
            message,
            podcast,
            episode,
            tlv,
            remote_podcast,
            remote_episode,
            payment_hash,
            payment_pubkey,
            payment_custom_key,
            payment_custom_value,
            payment_fee_msat,
            reply_to_idx
        )
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        ON CONFLICT(idx) DO UPDATE SET
            reply_to_idx = COALESCE(reply_to_idx, excluded.reply_to_idx)
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
            payment_info.payment_hash,
            payment_info.pubkey,
            payment_info.custom_key,
            payment_info.custom_value,
            payment_info.fee_msat,
            payment_info.reply_to_idx,
        ]
    )?;

    if let Some(reply_to_idx) = payment_info.reply_to_idx {
        mark_boost_as_replied(filepath, reply_to_idx)?;
    }

    Ok(true)
}

pub fn get_webhooks_from_db(filepath: &String, enabled: Option<bool>) -> Result<Vec<WebhookRecord>, Box<dyn Error>> {
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
            on_sent,
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
            on_sent: row.get(5)?,
            enabled: row.get(6)?,
            request_successful: row.get(7).ok(),
            request_timestamp: row.get(8).ok(),
        })
    }).unwrap();

    for row in rows {
        webhooks.push(row.unwrap());
    }

    Ok(webhooks)
}

pub fn load_webhook_from_db(filepath: &String, index: u64) -> Result<WebhookRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            url,
            token,
            on_boost,
            on_stream,
            on_sent,
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
            on_sent: row.get(5)?,
            enabled: row.get(6)?,
            request_successful: row.get(7).ok(),
            request_timestamp: row.get(8).ok(),
        })
    })?;

    Ok(webhook)
}

pub fn save_webhook_to_db(filepath: &String, webhook: &WebhookRecord) -> Result<u64, Box<dyn Error>> {
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
            on_sent,
            enabled,
            request_successful,
            request_timestamp
        )
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(idx) DO UPDATE SET
            url = excluded.url,
            token = excluded.token,
            on_boost = excluded.on_boost,
            on_stream = excluded.on_stream,
            on_sent = excluded.on_sent,
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
        webhook.on_sent,
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

pub fn set_webhook_last_request(filepath: &String, index: u64, successful: bool, timestamp: i64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(
        r#"UPDATE webhooks SET request_successful = ?2, request_timestamp = ?3 WHERE idx = ?1"#,
        params![index, successful, timestamp]
    )?;

    Ok(true)

}

pub fn delete_webhook_from_db(filepath: &String, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM webhooks WHERE idx = ?1"#, params![index])?;

    Ok(true)
}

pub fn load_settings_from_db(filepath: &String) -> Result<SettingsRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
             show_received_sats,
             show_split_percentage,
             hide_boosts,
             hide_boosts_below,
             play_pew,
             custom_pew_file
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
        }),
        Err(e) => Err(Box::new(e)),
    }
}

pub fn save_settings_to_db(filepath: &String, settings: &SettingsRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute(
        r#"INSERT INTO settings (
            idx,
            show_received_sats,
            show_split_percentage,
            hide_boosts,
            hide_boosts_below,
            play_pew,
            custom_pew_file
        )
        VALUES
            (1, ?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(idx) DO UPDATE SET
            show_received_sats = excluded.show_received_sats,
            show_split_percentage = excluded.show_split_percentage,
            hide_boosts = excluded.hide_boosts,
            hide_boosts_below = excluded.hide_boosts_below,
            play_pew = excluded.play_pew,
            custom_pew_file = excluded.custom_pew_file
        "#,
        params![
            settings.show_received_sats,
            settings.show_split_percentage,
            settings.hide_boosts,
            settings.hide_boosts_below,
            settings.play_pew,
            settings.custom_pew_file,
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to save settings".into())))
        }
    }
}

pub fn get_numerology_from_db(filepath: &String) -> Result<Vec<NumerologyRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut results: Vec<NumerologyRecord> = Vec::new();

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            amount,
            equality,
            emoji,
            sound_file,
            description
        FROM
            numerology
        "#
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(NumerologyRecord {
            index: row.get(0)?,
            amount: row.get(1)?,
            equality: row.get(2)?,
            emoji: row.get(3).ok(),
            sound_file: row.get(4).ok(),
            description: row.get(5).ok(),
        })
    }).unwrap();

    for row in rows {
        results.push(row.unwrap());
    }

    Ok(results)
}

pub fn load_numerology_from_db(filepath: &String, index: u64) -> Result<NumerologyRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            amount,
            equality,
            emoji,
            sound_file,
            description
        FROM
            numerology
        WHERE
            idx = :idx
        "#
    )?;

    let result = stmt.query_row(&[(":idx", index.to_string().as_str())], |row| {
        Ok(NumerologyRecord {
            index: row.get(0)?,
            amount: row.get(1)?,
            equality: row.get(2)?,
            emoji: row.get(3).ok(),
            sound_file: row.get(4).ok(),
            description: row.get(5).ok(),
        })
    })?;

    Ok(result)
}

pub fn save_numerology_to_db(filepath: &String, numero: &NumerologyRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let index = if numero.index > 0 {
        Some(numero.index)
    } else {
        None
    };

    match conn.execute(
        r#"INSERT INTO numerology (
            idx,
            amount,
            equality,
            emoji,
            sound_file,
            description
        )
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(idx) DO UPDATE SET
            amount = excluded.amount,
            equality = excluded.equality,
            emoji = excluded.emoji,
            sound_file = excluded.sound_file,
            description = excluded.description
        "#,
        params![
            index,
            numero.amount,
            numero.equality,
            numero.emoji,
            numero.sound_file,
            numero.description
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to save numerology".into())))
        }
    }
}

pub fn delete_numerology_from_db(filepath: &String, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM numerology WHERE idx = ?1"#, params![index])?;

    Ok(true)
}

pub fn reset_numerology_in_db(filepath: &String) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM numerology"#, [])?;
    insert_default_numerology(&conn)?;

    Ok(true)
}
