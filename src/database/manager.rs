use argon2::password_hash::SaltString;
use rusqlite::{Connection, Error, Result};
use rand::RngCore;

const SALT_LENGTH: usize = 32; // Recommended salt length (in bytes)

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; SALT_LENGTH];
    rand::rng().fill_bytes(&mut salt);
    salt
}

pub struct DatabaseManager {
    connection: Connection
}

impl DatabaseManager {
    pub fn new(database_path: &str) ->  Result<Self> {
        let connection = Connection::open(database_path).unwrap();
        Ok(DatabaseManager {connection})
    }

    pub fn create_master_table(&self) -> Result<()>  {
        println!("Creating master table...");
        self.connection.execute("CREATE TABLE IF NOT EXISTS master_table (\
        id INTEGER PRIMARY KEY AUTOINCREMENT,\
        salt BLOB NOT NULL);", []
        )?;

        println!("Created master table...");

        match self.insert_initial_salt() {
            Ok(_) => println!("Insertion was successful from main!"),
            Err(e) => eprintln!("Error during insertion: {}", e),
        }

        Ok(())
    }

    fn insert_initial_salt(&self) -> Result<()> {
        match  self.get_salt() {
            Ok(salt) => {
               Ok(())
            },
            Err(e) => {
                let salt = generate_salt();

                self.connection.execute(
                    "INSERT INTO master_table (salt) VALUES (?1);",
                    [&salt],
                )?;

                println!("Inserted master salt... {:?}", salt);
                Ok(())
            }
        }
    }

    pub fn get_salt(&self) -> Result<Vec<u8>> {
       let mut stmt = self.connection.prepare("SELECT salt FROM master_table LIMIT 1")?;
        let mut rows =  stmt.query([])?;
        if let Some(salt) = rows.next()? {
            let salt: Vec<u8> = salt.get(0)?;
            Ok(salt)
        }else {
            Err(Error::QueryReturnedNoRows)
        }
    }
}