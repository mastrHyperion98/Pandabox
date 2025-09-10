use std::rc::Rc;
use rusqlite::{Connection, Error, Result};

#[derive(Clone)]
pub struct DatabaseManager {
    connection: Rc<Connection>,  // Wrap Connection in Rc
}

impl DatabaseManager {
    pub fn new(database_path: &str) -> Result<Self> {
        let connection = Rc::new(Connection::open(database_path).unwrap());
        Ok(DatabaseManager { connection })
    }

    pub fn create_master_table(
        &self,
        salt: &Vec<u8>,
        encrypted_master: &Vec<u8>,
        nonce: &Vec<u8>,
    ) -> Result<()> {
        println!("Creating master table...");
        self.connection.execute(
            "CREATE TABLE IF NOT EXISTS master_table (\
        id INTEGER PRIMARY KEY AUTOINCREMENT,\
        encrypted_master_key BLOB NOT NULL,\
        nonce BLOB NOT NULL,\
        salt BLOB NOT NULL);",
            [],
        )?;

        println!("Created master table...");

        match self.insert_master_table(salt, encrypted_master, nonce) {
            Ok(_) => println!("Insertion was successful"),
            Err(e) => eprintln!("Error during insertion: {}", e),
        };

        Ok(())
    }
    
    pub fn check_master_table(&self) -> Result<bool> {
        let mut stmt = self.connection.prepare("SELECT * FROM master_table LIMIT 1")?;
        let mut rows = stmt.query([])?;

        if let Some(_) = rows.next()? {
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    fn insert_master_table(
        &self,
        salt: &Vec<u8>,
        encrypted_master: &Vec<u8>,
        nonce: &Vec<u8>,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO master_table (encrypted_master_key, nonce, salt) VALUES (?, ?, ?)",
            [encrypted_master, nonce, salt],
        )?;

        Ok(())
    }

    pub fn get_master_record(&self) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), Error> {
        let mut stmt = self
            .connection
            .prepare("SELECT encrypted_master_key, nonce, salt FROM master_table LIMIT 1")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            let encrypted_master_key: Vec<u8> = row.get(0)?;
            let nonce: Vec<u8> = row.get(1)?;
            let salt: Vec<u8> = row.get(2)?;
            Ok((encrypted_master_key, nonce, salt))
        } else {
            Err(Error::QueryReturnedNoRows) // Assuming you have a custom Error::QueryReturnedNoRows
        }
    }
}
