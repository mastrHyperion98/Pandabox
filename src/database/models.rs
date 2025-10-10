use diesel::prelude::*;
use crate::database::schema::{master_table, records};

/// Represents a record in the database.
#[derive(Queryable, Selectable)]
#[diesel(table_name = records)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Record {
    pub id: i32,
    pub service: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub notes: String,
}

/// Represents a new record to be inserted into the database.
#[derive(Insertable)]
#[diesel(table_name = records)]
pub struct NewRecord<'a> {
    pub service: &'a str,
    pub email: &'a str,
    pub username: &'a str,
    pub password: &'a str,
    pub notes: &'a str,
}

/// Represents a master record in the database.
#[derive(Queryable, Selectable)]
#[diesel(table_name = master_table)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct MasterRecord {
    pub id: i32,
    pub encrypted_master_key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub salt: Vec<u8>,
}

/// Represents a new master record to be inserted into the database.
#[derive(Insertable)]
#[diesel(table_name = master_table)]
pub struct NewMasterRecord<'a> {
    pub encrypted_master_key: &'a [u8],
    pub nonce: &'a [u8],
    pub salt: &'a [u8],
}
