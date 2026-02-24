use rusqlite::{params, Connection, Statement};
use std::error::Error;
use std::fmt;
use std::os::unix::fs::PermissionsExt;

mod boosts;
mod jwt;
mod node_info;
mod numerology;
mod sent_boosts;
mod settings;
mod triggers;

pub use boosts::*;
pub use jwt::*;
pub use node_info::*;
pub use numerology::*;
pub use sent_boosts::*;
pub use settings::*;
pub use triggers::*;

#[derive(Debug)]
struct HydraError(String);
impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}
impl Error for HydraError {}

//Connect to the database at the given file location
fn connect_to_database(init: bool, filepath: &str) -> Result<Connection, Box<dyn Error>> {
    if let Ok(conn) = Connection::open(filepath) {
        if init {
            match set_database_file_permissions(filepath) {
                Ok(_) => {},
                Err(e) => {
                    eprintln!("{:#?}", e);
                }
            }
            println!("Using database file: [{}]", filepath);
        }
        Ok(conn)
    } else {
        Err(Box::new(HydraError(format!("Could not open a database file at: [{}].", filepath))))
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
                    Err(Box::new(HydraError(format!("Error getting metadata from database file handle: [{}].  Error: {:#?}.", filepath, e))))
                }
            }
        },
        Err(e) => {
            Err(Box::new(HydraError(format!("Error opening database file handle: [{}] for permissions setting.  Error: {:#?}.", filepath, e))))
        }
    }
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool, Box<dyn Error>> {
    //Prepare and execute the query
    let mut stmt = conn.prepare(r#"SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1"#)?;
    let mut rows = stmt.query_map(params![table_name], |_| Ok(true))?;

    Ok(rows.next().is_some())
}

//Bind a query parameter by param name and desired value
fn bind_query_param(stmt: &mut Statement, name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    let idx = match stmt.parameter_index(name)? {
        Some(num) => num,
        None => {
            return Err(format!("{} param not found", name).into());
        }
    };

    stmt.raw_bind_parameter(idx, value)?;

    Ok(())
}

//Create or update a new database file if needed
pub fn create_database(filepath: &str) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(true, filepath)?;

    create_boosts_table(&conn)?;
    create_node_info_table(&conn)?;
    create_sent_boosts_table(&conn)?;
    create_numerology_table(&conn)?;
    create_settings_table(&conn)?;
    create_jwt_secret_table(&conn)?;
    create_triggers_table(&conn)?;

    // Migrate numeroloyg sounds and webhooks to triggers
    migrate_numerology_sounds_to_triggers(&conn)?;
    migrate_webhooks_to_triggers(&conn)?;

    Ok(true)
}

pub fn migrate_numerology_sounds_to_triggers(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    // Check if sound_file column still exists
    let has_sound_file: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('numerology') WHERE name = 'sound_file'",
            [],
            |row| row.get(0),
        )
        .map(|count: i64| count > 0)?;

    if !has_sound_file {
        return Ok(true);
    }

    let pos: i64 = conn
        .query_row("SELECT COALESCE(MAX(position), 0) FROM triggers", [], |row| row.get(0))
        .unwrap_or(0);

    println!("Migrating numerology sound files to triggers...");

    conn.execute(
        r#"INSERT INTO triggers (
            position, enabled,on_boost, on_stream, on_auto, on_sent, on_invoice,
            amount, amount_equality, sound_file, sound_name
        )
        SELECT
            ?1 + ROW_NUMBER() OVER (ORDER BY position),
            1, 1, 1, 1, 1, 1,
            amount, equality, sound_file, sound_file
        FROM numerology
        WHERE sound_file IS NOT NULL AND sound_file != ''
        ORDER BY idx"#,
        [pos],
    )?;

    conn.execute("ALTER TABLE numerology DROP COLUMN sound_file", [])?;

    println!("Migrated numerology sound files to triggers.");
    Ok(true)
}

pub fn migrate_webhooks_to_triggers(conn: &Connection) -> Result<bool, Box<dyn Error>> {
    if !table_exists(conn, "webhooks")? {
        return Ok(true);
    }

    let pos: i64 = conn
        .query_row("SELECT COALESCE(MAX(position), 0) FROM triggers", [], |row| row.get(0))
        .unwrap_or(0);

    println!("Migrating webhooks to triggers...");

    conn.execute(
        r#"INSERT INTO triggers (
            position,
            enabled, on_boost, on_stream, on_sent, on_auto, on_invoice,
            amount, amount_equality,
            webhook_url, webhook_token, webhook_successful, webhook_timestamp
        ) SELECT
            ?1 + ROW_NUMBER() OVER (ORDER BY idx),
            enabled, on_boost, on_stream, on_sent, 0, 0,
            amount, equality,
            url, token, request_successful, request_timestamp
        FROM webhooks
        ORDER BY idx"#,
        [pos],
    )?;

    conn.execute("ALTER TABLE webhooks RENAME TO webhooks_archive", [])?;

    println!("Migrated webhooks to triggers.");
    Ok(true)
}