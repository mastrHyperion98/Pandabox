// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::encrypt::cryptography::CryptEngine;
use base64::Engine;
use chacha20poly1305::Error as ChaChaError;
use dirs::home_dir;
use slint::{PlatformError, SharedString, Window};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

mod database;
mod encrypt;
mod handler;

slint::include_modules!();

const APP_NAME: &str = "RustPasswordManager";

// Assume you have a function to write to your database
fn on_authenticate(
    data: SharedString,
    path: String,
    window: &Window,
) -> Result<(), Box<dyn Error>> {
    // Now we create the database
    let manager = database::manager::DatabaseManager::new(path.as_str()).unwrap();
    let (key, nonce, salt) = manager.get_master_record()?;

    let engine = CryptEngine::new(data.as_str(), &salt).unwrap();
    match engine.decrypt_master_key(nonce.as_slice(), key.as_ref()) {
        Ok(_) => {
            println!("Master key successfully decrypted");
            window.hide().unwrap();
        }
        Err(_) => {
            println!("Failed to decrypt master key");
            return Err(From::from("Failed to decrypt master key"));
        }
    };

    // In a real application, you would have your database interaction logic here
    Ok(())
}

fn create_db(data: SharedString, path: String, window: &Window) -> Result<(), ChaChaError> {
    // Before creating the database perhaps we should create the salt, nonce and encyrption key
    let salt = CryptEngine::generate_salt();
    let engine = CryptEngine::new(data.as_str(), &salt).unwrap();
    let master_key = CryptEngine::generate_master_key();
    let (nonce, ciphertext) = engine.encrypt_master_key(master_key.as_ref())?;

    // Now we create the database
    let manager = database::manager::DatabaseManager::new(path.as_str()).unwrap();

    println!("Creating DB");
    match manager.create_master_table(salt.as_ref(), ciphertext.as_ref(), nonce.as_ref()) {
        Ok(_) => {
            println!("Created Master Table");
            match AuthenticateWindow::new() {
                Ok(ui) => {
                    let weak = ui.as_weak();
                    handler::setup_authentication_handler(weak.clone(), path.clone());

                    match ui.run() {
                        _ => {
                            window.hide();
                        }
                    };
                }
                Err(_) => {}
            };
        }
        Err(_) => {
            println!("Failed to create Master Table");
        }
    };
    Ok(())
}

fn get_user_db_path_cross_platform(db_filename: &str) -> Option<PathBuf> {
    if let Some(mut home) = home_dir() {
        home.push(APP_NAME);
        home.push(db_filename);
        Some(home)
    } else {
        eprintln!("Warning: Could not determine the user's home directory.");
        None
    }
}

fn check_db_exist() -> (bool, String) {
    let db_file = "db.sqlite";

    if let Some(db_path) = get_user_db_path_cross_platform(db_file) {
        println!("Potential database path: {}", db_path.display());
        if let Some(parent_dir) = db_path.parent() {
            if !parent_dir.exists() {
                println!("Directory {:?} does not exist. Creating...", parent_dir);
                fs::create_dir_all(parent_dir).expect("TODO: panic message");
            }
        }

        if db_path.exists() {
            println!("Database file exists at: {}", db_path.display());
            let path_str = db_path.to_str().unwrap_or_default().to_owned();
            (true, path_str)
        } else {
            println!("Database file does not exist at: {}", db_path.display());
            let path_str = db_path.to_str().unwrap_or_default().to_owned();
            (false, path_str)
        }
    } else {
        println!("Could not determine the user's home directory.");
        (false, db_file.to_string())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (db_exist, path) = check_db_exist();
    run(db_exist, path)?;
    Ok(())
}

fn run(db_exist: bool, path: String) -> Result<(), Box<dyn Error>> {
    get_initial_ui(db_exist, path)?;
    Ok(())
}

//TODO: Have this return the ui and have it execute ui.run in the run function of this main program
fn get_initial_ui(db_exist: bool, path: String) -> Result<(), Box<dyn Error>> {
    if db_exist {
        let ui = AuthenticateWindow::new()?;
        let weak = ui.as_weak();

        handler::setup_authentication_handler(weak.clone(), path.clone());

        ui.run()?;
        Ok(())
    } else {
        let ui = CreateDbWindow::new()?;
        let weak = ui.as_weak();

        handler::setupt_createdb_handler(weak.clone(), path.clone());

        ui.run()?;
        Ok(())
    }
}
