use rusqlite::{params, Connection};
use std::error::Error;
use std::fmt;
use serde::{Deserialize, Serialize};

pub const SQLITE_FILE: &str = "database.db";

#[derive(Serialize, Deserialize, Debug)]
pub struct BoostRecord {
    pub index: u64,
    pub time: i64,
    pub value_msat: i64,
    pub message: String,
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


//Create a new database file
pub fn create_database() -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE)?;

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS boosts (
             idx integer primary key,
             time integer,
             value_msat integer,
             message text,
             tlv text
         )",
        [],
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to create database: [{}].", SQLITE_FILE).into())))
        }
    }
}


//Add an invoice to the database
pub fn add_invoice_to_db(boost: BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE)?;

    match conn.execute("INSERT INTO boosts (idx, time, value_msat, message, tlv) VALUES (?1, ?2, ?3, ?4, ?5)",
                       params![boost.index, boost.time, boost.value_msat, boost.message, boost.tlv]) {
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
pub fn get_boosts_from_db(index: u64, max: u64) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();

    //Prepare and execute the query
    let mut stmt = conn.prepare("SELECT idx, time, value_msat, message, tlv \
                                 FROM boosts \
                                 WHERE idx >= :index \
                                 ORDER BY idx ASC LIMIT :max")?;
    let rows = stmt.query_map(&[(":index", index.to_string().as_str()), (":max", max.to_string().as_str())], |row| {
        Ok(BoostRecord {
            index: row.get(0)?,
            time: row.get(1)?,
            value_msat: row.get(2)?,
            message: row.get(3)?,
            tlv: row.get(4)?,
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
pub fn get_last_boost_index_from_db() -> Result<u64, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE)?;
    let mut boosts: Vec<BoostRecord> = Vec::new();
    let max = 1;

    //Prepare and execute the query
    let mut stmt = conn.prepare("SELECT idx, time, value_msat, message, tlv \
                                 FROM boosts \
                                 ORDER BY idx DESC LIMIT :max")?;
    let rows = stmt.query_map(&[(":max", max.to_string().as_str())], |row| {
        Ok(BoostRecord {
            index: row.get(0)?,
            time: row.get(1)?,
            value_msat: row.get(2)?,
            message: row.get(3)?,
            tlv: row.get(4)?,
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