use rusqlite::{params, Connection};
use std::error::Error;
use std::collections::HashMap;

use crate::{
    connect_to_database,
    HydraError,
    BoostRecord,
    PaymentRecord,
    BoostFilters,
    mark_boost_as_replied,
    bind_query_param,
    ActionType,
    ListType,
};

pub fn create_sent_boosts_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
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
            return Err(Box::new(HydraError("Failed to create database sent_boosts table.".into())))
        }
    }

    Ok(true)
}

//Get all of the sent boosts from the database
pub fn get_payments_from_db(filepath: &str, index: u64, max: u64, direction: bool, escape_html: bool, filters: BoostFilters) -> Result<Vec<BoostRecord>, Box<dyn Error>> {
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

    //Parse the results
    while let Some(row) = rows.next()? {
        let boost = BoostRecord {
            index: row.get(0)?,
            time: row.get(1)?,
            value_msat: row.get(2)?,
            value_msat_total: row.get(3)?,
            action: ActionType::from_u8(row.get(4)?),
            list_type: ListType::Sent,
            sender: row.get(5)?,
            app: row.get(6)?,
            message: row.get(7)?,
            podcast: row.get(8)?,
            episode: row.get(9)?,
            tlv: row.get(10)?,
            remote_podcast: row.get(11).ok(),
            remote_episode: row.get(12).ok(),
            reply_sent: false,
            custom_key: None,
            custom_value: None,
            memo: None,
            payment_info: Some(PaymentRecord {
                payment_hash: row.get(13)?,
                pubkey: row.get(14)?,
                custom_key: row.get(15)?,
                custom_value: row.get(16)?,
                fee_msat: row.get(17)?,
                reply_to_idx: row.get(18)?,
            }),
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

pub fn get_last_payment_index_from_db(filepath: &str) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare("SELECT MAX(idx) FROM sent_boosts")?;
    let index = stmt.query_row([], |row| row.get(0))?;

    if let Some(idx) = index {
        return Ok(idx);
    }

    Ok(0)
}

//Add a payment (sent boost) to the database
pub fn add_payment_to_db(filepath: &str, boost: &BoostRecord) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let payment_info = match &boost.payment_info {
        Some(info) => info,
        None => {
            return Err(Box::new(HydraError(format!("Missing payment info for sent boost: [{}].", boost.index))))
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
            boost.action as u8,
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

//Get podcasts that were send boosts from this node
pub fn get_sent_podcasts_from_db(filepath: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let query = "SELECT DISTINCT podcast FROM sent_boosts WHERE podcast <> '' ORDER BY podcast".to_string();

    let mut stmt = conn.prepare(&query)?;
    let mut rows = stmt.raw_query();

    //Parse the results
    let mut podcasts = Vec::new();

    while let Some(row) = rows.next()? {
        podcasts.push(row.get(0)?);
    }

    Ok(podcasts)
}
