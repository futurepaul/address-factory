use anyhow::Result;
use chrono::Local;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct Database {
    connection: Connection,
    pub filename: String,
}

#[derive(Debug)]
pub struct Entry {
    id: i32,
    address: String,
    message: String,
}

impl Entry {
    pub fn new(address: &str, signed_message: &str) -> Self {
        Self {
            id: 0,
            address: address.to_string(),
            message: signed_message.to_string(),
        }
    }
}

impl Database {
    // Create SQLite database of addresses & signed messages

    pub fn new() -> Result<Self> {
        let date_time = Local::now().format("%Y-%m-%d_%H-%M").to_string();
        let filename = format!("{}_signed_addresses.db", date_time);

        // TODO: fail gracefully if the filename already exists
        let connection = Connection::open(filename.clone())?;

        connection.execute(
            "CREATE TABLE entries (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  address         TEXT NOT NULL,
                  message         TEXT NOT NULL
                  )",
            params![],
        )?;
        Ok(Self {
            connection,
            filename,
        })
    }

    pub fn insert(&self, entry: Entry) -> Result<()> {
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
