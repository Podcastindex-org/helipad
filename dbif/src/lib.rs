use rusqlite::{params, Connection};
use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::os::unix::fs::PermissionsExt;


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
    pub pubkey: String,
    pub custom_key: u64,
    pub custom_value: String,
    pub fee_msat: i64,
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

    //Create the boost_remote_items table
    match conn.execute("ALTER TABLE boosts ADD COLUMN remote_podcast text", []) {
        Ok(_) => {
            println!("Boosts remote podcast column added.");
        }
        Err(_) => {}
    }

    //Create the boost_remote_items table
    match conn.execute("ALTER TABLE boosts ADD COLUMN remote_episode text", []) {
        Ok(_) => {
            println!("Boosts remote episode column added.");
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
             payment_pubkey text,
             payment_custom_key integer,
             payment_custom_value text,
             payment_fee_msat integer
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

    Ok(true)
}


//Add an invoice to the database
pub fn add_invoice_to_db(filepath: &String, boost: BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute("INSERT INTO boosts (idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode) \
                                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
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
                                       boost.remote_episode]
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


//Get all of the boosts from the database
pub fn get_boosts_from_db(filepath: &String, index: u64, max: u64, direction: bool, escape_html: bool) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();

    let mut ltgt = ">=";
    if direction {
        ltgt = "<=";
    }

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
                                       remote_episode \
                                 FROM boosts \
                                 WHERE action = 2 \
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


//Get all of the boosts from the database
pub fn get_streams_from_db(filepath: &String, index: u64, max: u64, direction: bool, escape_html: bool) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();


    let mut ltgt = ">=";
    if direction {
        ltgt = "<=";
    }

    //Build the query
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
                                       remote_episode \
                                 FROM boosts \
                                 WHERE action = 1 \
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
    let mut stmt = conn.prepare("SELECT idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode \
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
            payment_pubkey,
            payment_custom_key,
            payment_custom_value,
            payment_fee_msat
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
            payment_info: Some(PaymentRecord {
                pubkey: row.get(13)?,
                custom_key: row.get(14)?,
                custom_value: row.get(15)?,
                fee_msat: row.get(16)?,
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
pub fn add_payment_to_db(filepath: &String, boost: BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let payment_info = match boost.payment_info {
        Some(info) => info,
        None => {
            return Err(Box::new(HydraError(format!("Missing payment info for sent boost: [{}].", boost.index).into())))
        }
    };

    match conn.execute(
        "INSERT INTO sent_boosts
            (idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv, remote_podcast, remote_episode, payment_pubkey, payment_custom_key, payment_custom_value, payment_fee_msat)
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
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
            payment_info.pubkey,
            payment_info.custom_key,
            payment_info.custom_value,
            payment_info.fee_msat,
        ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to add sent boost: [{}].", boost.index).into())))
        }
    }
}