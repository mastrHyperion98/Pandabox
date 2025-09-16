use std::path::PathBuf;
use std::fs;
use std::env;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;

use crate::database::models::{MasterRecord, NewMasterRecord, NewRecord, Record};
use crate::database::schema::{master_table};
use crate::database::schema::records::dsl::*;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Clone)]
pub struct DatabaseManager {
    database_url: String,
}

impl DatabaseManager {
    pub fn new() -> Self {
        dotenv().ok();
        
        // Get the home directory and create the application directory
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
            
        let app_dir = home_dir.join("Pandabox");
        fs::create_dir_all(&app_dir).expect("Failed to create application directory");
        
        // Set the database path
        let db_path = app_dir.join("pandabox.db");
        let database_url = db_path.to_str().expect("Invalid database path").to_string();
        
        // Set the DATABASE_URL environment variable for Diesel CLI
        std::env::set_var("DATABASE_URL", &database_url);
        
        // Create or connect to the database
        let mut connection = SqliteConnection::establish(&database_url)
            .unwrap_or_else(|e| panic!("Error connecting to {}: {}", database_url, e));

        // Run migrations
        connection
            .run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        DatabaseManager { database_url }
    }

    fn establish_connection(&self) -> SqliteConnection {
        SqliteConnection::establish(&self.database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", self.database_url))
    }

    pub fn create_master_record(
        &self,
        salt: &[u8],
        encrypted_master: &[u8],
        nonce: &[u8],
    ) -> QueryResult<usize> {
        let mut connection = self.establish_connection();
        let new_master_record = NewMasterRecord {
            salt,
            encrypted_master_key: encrypted_master,
            nonce,
        };

        diesel::insert_into(master_table::dsl::master_table)
            .values(&new_master_record)
            .execute(&mut connection)
    }

    pub fn check_master_table_exists(&self) -> QueryResult<bool> {
        use crate::database::schema::master_table::dsl::*;

        let mut connection = self.establish_connection();
        let count = master_table
            .limit(1)
            .count()
            .get_result::<i64>(&mut connection)?;
        Ok(count > 0)
    }

    pub fn get_master_record(&self) -> QueryResult<MasterRecord> {
        use crate::database::schema::master_table::dsl::*;

        let mut connection = self.establish_connection();
        master_table.first(&mut connection)
    }

    pub fn insert_entry(
        &self,
        service_name: &str,
        email_str: &str,
        username_str: &str,
        password_str: &str,
        notes_str: &str,
    ) -> QueryResult<Record> {


        let mut connection = self.establish_connection();
        let new_record = NewRecord {
            service: service_name,
            email: email_str,
            username: username_str,
            password: password_str,
            notes: notes_str,
        };

        // Insert the record
        diesel::insert_into(records)
            .values(&new_record)
            .execute(&mut connection)?;

        // Get the ID of the last inserted record
        let last_id: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("last_insert_rowid()"))
            .get_result(&mut connection)?;

        // Fetch the complete record
        records.find(last_id).first(&mut connection)
    }
    
    pub fn get_all_records(&self) -> QueryResult<Vec<Record>> {
        use crate::database::schema::records::dsl::*;
        
        let mut connection = self.establish_connection();
        records.load::<Record>(&mut connection)
    }

    pub fn update_record(
        &self,
        record_id: i32,
        service_name: &str,
        email_str: &str,
        username_str: &str,
        password_str: &str,
        notes_str: &str,
    ) -> QueryResult<usize> {
        use crate::database::schema::records::dsl::*;

        let mut connection = self.establish_connection();
        diesel::update(records.find(record_id))
            .set((
                service.eq(service_name),
                email.eq(email_str),
                username.eq(username_str),
                password.eq(password_str),
                notes.eq(notes_str),
            ))
            .execute(&mut connection)
    }
}
