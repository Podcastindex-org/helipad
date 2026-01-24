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
mod webhooks;

pub use boosts::*;
pub use jwt::*;
pub use node_info::*;
pub use numerology::*;
pub use sent_boosts::*;
pub use settings::*;
pub use webhooks::*;

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
pub fn create_database(filepath: &String) -> Result<bool, Box<dyn Error>> {
    let conn = connect_to_database(true, filepath)?;

    create_boosts_table(&conn)?;
    create_node_info_table(&conn)?;
    create_sent_boosts_table(&conn)?;
    create_numerology_table(&conn)?;
    create_settings_table(&conn)?;
    create_webhooks_table(&conn)?;
    create_jwt_secret_table(&conn)?;

    Ok(true)
}
