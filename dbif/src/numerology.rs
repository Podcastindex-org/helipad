use rusqlite::{Connection, params};
use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::{connect_to_database, HydraError, table_exists};

#[derive(Serialize, Deserialize, Debug)]
pub struct NumerologyRecord {
    pub index: u64,
    pub position: u64,
    pub amount: u64,
    pub equality: String,
    pub emoji: Option<String>,
    pub description: Option<String>,
}

pub fn create_numerology_table(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    //Create the numerology table
    let numerology_exists = table_exists(&conn, "numerology")?;

    match conn.execute(
        "CREATE TABLE IF NOT EXISTS numerology (
             idx integer primary key,
             position integer,
             equality text not null,
             amount integer not null,
             emoji text,
             description text
         )",
        [],
    ) {
        Ok(_) => {
            println!("Numerology table is ready.");
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError("Failed to create database numerology table.".into())))
        }
    }

    if !numerology_exists && insert_default_numerology(&conn)? {
        println!("Default numerology added.");
    }

    Ok(true)
}

pub fn get_numerology_from_db(filepath: &str) -> Result<Vec<NumerologyRecord>, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;
    let mut results: Vec<NumerologyRecord> = Vec::new();

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            position,
            amount,
            equality,
            emoji,
            description
        FROM
            numerology
        ORDER BY
            position
        "#
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(NumerologyRecord {
            index: row.get(0)?,
            position: row.get(1)?,
            amount: row.get(2)?,
            equality: row.get(3)?,
            emoji: row.get(4).ok(),
            description: row.get(5).ok(),
        })
    }).unwrap();

    for row in rows {
        results.push(row.unwrap());
    }

    Ok(results)
}

pub fn load_numerology_from_db(filepath: &str, index: u64) -> Result<NumerologyRecord, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let mut stmt = conn.prepare(
        r#"SELECT
            idx,
            position,
            amount,
            equality,
            emoji,
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
            position: row.get(1)?,
            amount: row.get(2)?,
            equality: row.get(3)?,
            emoji: row.get(4).ok(),
            description: row.get(5).ok(),
        })
    })?;

    Ok(result)
}

pub fn save_numerology_to_db(filepath: &str, numero: &NumerologyRecord) -> Result<u64, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    let index = if numero.index > 0 {
        Some(numero.index)
    } else {
        None
    };

    set_numerology_position_in_db(filepath, numero.index, numero.position)?;

    let mut stmt = conn.prepare(
        r#"INSERT INTO numerology (
            idx,
            position,
            amount,
            equality,
            emoji,
            description
        )
        VALUES
            (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(idx) DO UPDATE SET
            position = excluded.position,
            amount = excluded.amount,
            equality = excluded.equality,
            emoji = excluded.emoji,
            description = excluded.description
        RETURNING idx
        "#,
    )?;

    let params = params![
        index,
        numero.position,
        numero.amount,
        numero.equality,
        numero.emoji,
        numero.description
    ];

    let idx = stmt.query_row(params, |row| {
        let idx: u64 = row.get(0)?;
        Ok(idx)
    })?;

    renumber_numerology_positions_in_db(filepath)?;

    Ok(idx)
}

pub fn set_numerology_position_in_db(filepath: &str, index: u64, position: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    #[allow(clippy::comparison_chain)]
    if index > 0 {
        let current = load_numerology_from_db(filepath, index)?;

        if position < current.position {
            // shift items between the old and new position down by 1
            conn.execute(
                r#"UPDATE numerology SET position = position + 1 WHERE position >= ? AND position <= ? AND idx <> ?"#,
                params![
                    position,
                    current.position,
                    index,
                ]
            )?;
        }
        else if position > current.position {
            // shift items between the old and new position up by 1
            conn.execute(
                r#"UPDATE numerology SET position = position - 1 WHERE position <= ? AND position >= ? AND idx <> ?"#,
                params![
                    position,
                    current.position,
                    index,
                ]
            )?;
        }
    }
    else {
        // shift items down by 1
        conn.execute(
            r#"UPDATE numerology SET position = position + 1 WHERE position >= ?"#,
            params![
                position,
            ]
        )?;
    }


    Ok(true)
}

pub fn renumber_numerology_positions_in_db(filepath: &str) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    // renumber all positions
    conn.execute(
        r#"UPDATE numerology SET position = (SELECT COUNT(*) FROM numerology b WHERE b.position < numerology.position) + 1"#,
        []
    )?;

    Ok(true)
}

pub fn delete_numerology_from_db(filepath: &str, index: u64) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM numerology WHERE idx = ?1"#, params![index])?;

    renumber_numerology_positions_in_db(filepath)?;

    Ok(true)
}

pub fn reset_numerology_in_db(filepath: &str) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(false, filepath)?;

    conn.execute(r#"DELETE FROM numerology"#, [])?;
    insert_default_numerology(&conn)?;

    Ok(true)
}

pub fn insert_default_numerology(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    let queries = vec![
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (1, 'Satchel of Richards Donation x 7', '🍆🍆🍆🍆🍆🍆🍆', '1111111', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (2, 'Satchel of Richards Donation x 6', '🍆🍆🍆🍆🍆🍆', '111111', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (3, 'Satchel of Richards Donation x 5', '🍆🍆🍆🍆🍆', '11111', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (4, 'Satchel of Richards Donation x 4', '🍆🍆🍆🍆', '1111', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (5, 'Satchel of Richards Donation x 3', '🍆🍆🍆', '111', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (6, 'Satchel of Richards Donation x 2', '🍆🍆', '11', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (7, 'Ducks In a Row Donation x 7', '🦆🦆🦆🦆🦆🦆🦆', '2222222', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (8, 'Ducks In a Row Donation x 6', '🦆🦆🦆🦆🦆🦆', '222222', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (9, 'Ducks In a Row Donation x 5', '🦆🦆🦆🦆🦆', '22222', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (10, 'Ducks In a Row Donation x 4', '🦆🦆🦆🦆', '2222', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (11, 'Ducks In a Row Donation x 3', '🦆🦆🦆', '222', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (12, 'Ducks In a Row Donation x 2', '🦆🦆', '22', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (13, 'Swan Donation x 7', '🦢🦢🦢🦢🦢🦢🦢', '5555555', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (14, 'Swan Donation x 6', '🦢🦢🦢🦢🦢🦢', '555555', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (15, 'Swan Donation x 5', '🦢🦢🦢🦢🦢', '55555', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (16, 'Swan Donation x 4', '🦢🦢🦢🦢', '5555', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (17, 'Swan Donation x 3', '🦢🦢🦢', '555', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (18, 'Swan Donation x 2', '🦢🦢', '55', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (19, 'Countdown Donation x 5', '💥💥💥💥💥', '7654321', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (20, 'Countdown Donation x 4', '💥💥💥💥', '654321', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (21, 'Countdown Donation x 3', '💥💥💥', '54321', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (22, 'Countdown Donation x 2', '💥💥', '4321', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (23, 'Countdown Donation', '💥', '321', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (24, 'Countup Donation x 5', '🧛🧛🧛🧛🧛', '1234567', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (25, 'Countup Donation x 4', '🧛🧛🧛🧛', '123456', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (26, 'Countup Donation x 3', '🧛🧛🧛', '12345', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (27, 'Countup Donation x 2', '🧛🧛', '1234', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (28, 'Countup Donation', '🧛', '123', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (29, 'Bowler Donation x 3 +🦃', '🎳🎳🎳🦃', '101010', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (30, 'Bowler Donation x 2', '🎳🎳', '1010', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (31, 'Bowler Donation', '🎳', '10', '=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (32, 'Dice Donation', '🎲', '11', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (33, 'Bitcoin donation', '🪙', '21', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (34, 'Magic Number Donation', '✨', '33', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (35, 'Swasslenuff Donation', '💋', '69', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (36, 'Greetings Donation', '👋', '73', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (37, 'Love and Kisses Donation', '🥰', '88', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (38, 'Stoner Donation', '✌👽💨', '420', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (39, 'Devil Donation', '😈', '666', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (40, 'Angel Donation', '😇', '777', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (41, 'America Fuck Yeah Donation', '🇺🇸', '1776', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (42, 'Canada Donation', '🇨🇦', '1867', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (43, 'Boobs Donation', '🎱🎱', '6006', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (44, 'Boobs Donation', '🎱🎱', '8008', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (45, 'Wolf Donation', '🐺', '9653', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (46, 'Boost Donation', '🔁', '30057', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (47, 'Pi Donation x 5', '🥧🥧🥧🥧🥧', '3141592', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (48, 'Pi Donation x 4', '🥧🥧🥧🥧', '314159', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (49, 'Pi Donation x 3', '🥧🥧🥧', '31415', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (50, 'Pi Donation x 2', '🥧🥧', '3141', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (51, 'Pi Donation', '🥧', '314', '=~')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (52, 'Poo donation', '💩', '9', '<')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (53, 'Lit donation 100k', '🔥', '100000', '>=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (54, 'Lit donation 50k', '🔥', '50000', '>=')",
        "INSERT INTO numerology (position, description, emoji, amount, equality) VALUES (55, 'Lit donation 10k', '🔥', '10000', '>=')",
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
