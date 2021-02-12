use anyhow::Result;
use chrono::Local;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct Database {
    connection: Connection,
}

#[derive(Debug)]
struct Entry {
    id: i32,
    address: String,
    message: String,
}

impl Database {
    pub fn new() -> Result<Self> {
        let date_time = Local::now().format("%Y-%m-%d_%H-%M").to_string();
        let filename = format!("{}_signed_addresses.db", date_time);

        // TODO: fail gracefully if the filename already exists
        let conn = Connection::open(filename)?;

        conn.execute(
            "CREATE TABLE entries (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  address         TEXT NOT NULL,
                  message         TEXT NOT NULL
                  )",
            params![],
        )?;
        Ok(Self { connection: conn })
    }

    pub fn insert(&self, address: &str, signed_message: &str) -> Result<()> {
        let entry = Entry {
            id: 0,
            address: address.to_string(),
            message: signed_message.to_string(),
        };

        self.connection.execute(
            "INSERT INTO entries (address, message) VALUES (?1, ?2)",
            params![entry.address, entry.message],
        )?;

        Ok(())
    }

    pub fn print_entries(&self) -> Result<()> {
        let mut stmt = self
            .connection
            .prepare("SELECT id, address, message FROM entries")?;
        let entry_itr = stmt.query_map(params![], |row| {
            Ok(Entry {
                id: row.get(0)?,
                address: row.get(1)?,
                message: row.get(2)?,
            })
        })?;

        for entry in entry_itr {
            println!("Found entry {:?}", entry.unwrap());
        }

        Ok(())
    }
}
