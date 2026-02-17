use rusqlite::{Connection, params};
use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::{connect_to_database, HydraError};

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfoRecord {
    pub lnd_alias: String,
    pub node_pubkey: String,
    pub node_version: String,
}

pub fn create_node_info_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
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
            return Err(Box::new(HydraError("Failed to create database node_info table.".into())))
        }
    }
    Ok(true)
}

pub fn get_node_info_from_db(filepath: &str) -> Result<NodeInfoRecord, Box<dyn Error>> {
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

    let mut rows = stmt.query_map([], |row| {
        Ok(NodeInfoRecord {
            lnd_alias: row.get(0)?,
            node_pubkey: row.get(1)?,
            node_version: row.get(2)?,
        })
    })?;

    // Return first record if found
    if let Some(row) = rows.next() {
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
pub fn add_node_info_to_db(filepath: &str, info: NodeInfoRecord) -> Result<bool, Box<dyn Error>> {
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
            Err(Box::new(HydraError("Failed to add node info".into())))
        }
    }
}

//Set/Get the wallet balance from the database in sats
pub fn add_wallet_balance_to_db(filepath: &str, balance: i64) -> Result<bool, Box<dyn Error>> {
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
            Err(Box::new(HydraError(format!("Failed to update wallet balance in database: [{}].", balance))))
        }
    }
}
pub fn get_wallet_balance_from_db(filepath: &str) -> Result<i64, Box<dyn Error>> {
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