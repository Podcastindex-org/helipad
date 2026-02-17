use rusqlite::{Connection, params};
use std::error::Error;
use rand::{distr::Alphanumeric, Rng};
use crate::{connect_to_database, HydraError};

pub fn create_jwt_secret_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    //Create the jwt_secret table
    match conn.execute(
        "CREATE TABLE IF NOT EXISTS jwt_secret (
             idx integer primary key,
             secret text not null,
             created_at integer not null
         )",
        [],
    ) {
        Ok(_) => {
            println!("JWT secret table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to create database jwt_secret table.".into())))
        }
    }

    Ok(true)
}

//Get the JWT secret from the database
pub fn get_or_create_jwt_secret(filepath: &str) -> Result<String, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare("SELECT secret FROM jwt_secret WHERE idx = 1")?;

    let result = stmt.query_row([], |row| {
        row.get(0)
    });

    if let Ok(secret) = result {
        return Ok(secret);
    }

    //If no secret found, generate a new one
    let secret: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect();

    //Set the new secret in the database
    let _ = set_jwt_secret(filepath, &secret);

    Ok(secret)
}

//Set the JWT secret in the database
pub fn set_jwt_secret(filepath: &str, secret: &str) -> Result<(), Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let timestamp = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT OR REPLACE INTO jwt_secret (idx, secret, created_at) VALUES (1, ?1, ?2)",
        params![secret, timestamp],
    )?;

    Ok(())
}
