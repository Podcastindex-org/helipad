use rusqlite::{params, Connection};
use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;


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
}


#[derive(Debug)]
struct HydraError(String);
impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}
impl Error for HydraError {}


fn connect_to_database(init: bool, filepath: &Path) -> Result<Connection, Box<dyn Error>> {
    if let Ok(conn) = Connection::open(filepath) {
        if init {
            match set_database_file_permissions(filepath) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("{:#?}", e);
                }
            }
            println!("Using database file: [{}]", filepath.display());
        }
        Ok(conn)
    } else {
        return Err(Box::new(HydraError(format!("Could not open a database file at: [{}].", filepath.display()).into())))
    }
}


//Set permissions on the database file
fn set_database_file_permissions(filepath: &Path) -> Result<bool, Box<dyn Error>> {

    match std::fs::File::open(filepath) {
        Ok(fh) => {
            match fh.metadata() {
                Ok(metadata) => {
                    let mut perms = metadata.permissions();
                    perms.set_mode(0o666);
                    println!("Set file permission to: [666] on database file: [{}]", filepath.display());
                    Ok(true)
                },
                Err(e) => {
                    return Err(Box::new(HydraError(format!("Error getting metadata from database file handle: [{}].  Error: {:#?}.", filepath.display(), e).into())))
                }
            }
        },
        Err(e) => {
            return Err(Box::new(HydraError(format!("Error opening database file handle: [{}] for permissions setting.  Error: {:#?}.", filepath.display(), e).into())))
        }
    }
}


//Create a new database file if needed
pub fn create_database(filepath: &Path) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(true, filepath)?;

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
             tlv text
         )",
        [],
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to create database: [{}].", filepath.display()).into())))
        }
    }
}


//Add an invoice to the database
pub fn add_invoice_to_db(filepath: &Path, boost: BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    match conn.execute("INSERT INTO boosts (idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv) \
                                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                                       boost.tlv]
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
pub fn get_boosts_from_db(filepath: &Path, index: u64, max: u64, direction: bool) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
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
                                       tlv \
                                 FROM boosts \
                                 WHERE action = 2 \
                                   AND idx {} :index \
                                 ORDER BY idx ASC \
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
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let boost: BoostRecord = row.unwrap();
        boosts.push(boost);
    }

    Ok(boosts)
}


//Get the last boost index number from the database
pub fn get_last_boost_index_from_db(filepath: &Path) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();
    let max = 1;

    //Prepare and execute the query
    let mut stmt = conn.prepare("SELECT idx, time, value_msat, value_msat_total, action, sender, app, message, podcast, episode, tlv \
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
