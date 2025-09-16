-- Your SQL goes here

CREATE TABLE master_table (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    encrypted_master_key BLOB NOT NULL,
    nonce BLOB NOT NULL,
    salt BLOB NOT NULL
);
