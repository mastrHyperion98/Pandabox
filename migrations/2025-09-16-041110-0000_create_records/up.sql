-- Your SQL goes here

CREATE TABLE records (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    service TEXT NOT NULL,
    email TEXT NOT NULL,
    username TEXT NOT NULL,
    password TEXT NOT NULL,
    notes TEXT NOT NULL
);
