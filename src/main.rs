// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::encrypt::cryptography::CryptEngine;
use base64::Engine;
use dirs::home_dir;
use slint::{ SharedString};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

mod database;
mod encrypt;
slint::include_modules!();

const APP_NAME: &str = "RustPasswordManager";

// Assume you have a function to write to your database
fn on_authenticate(
    data: SharedString,
    path: String,
) -> bool {
    // Now we create the database
    let manager = database::manager::DatabaseManager::new(path.as_str()).unwrap();
    let (key, nonce, salt) = manager.get_master_record().unwrap();

    let engine = CryptEngine::new(data.as_str(), &salt).unwrap();
    let mut status = false;
    match engine.decrypt_master_key(nonce.as_slice(), key.as_ref()) {
        Ok(_) => {
            println!("Master key successfully decrypted");
            status=true;
        }
        Err(_) => {
            println!("Failed to decrypt master key");
        }
    };

    status

}

fn create_db(data: SharedString, path: String) -> bool {
    // Before creating the database perhaps we should create the salt, nonce and encyrption key
    let salt = CryptEngine::generate_salt();
    let engine = CryptEngine::new(data.as_str(), &salt).unwrap();
    let master_key = CryptEngine::generate_master_key();
    let (nonce, ciphertext) = engine.encrypt_master_key(master_key.as_ref()).unwrap();

    // Now we create the database
    let manager = database::manager::DatabaseManager::new(path.as_str()).unwrap();

    println!("Creating DB");
    let mut status = false;
    match manager.create_master_table(salt.as_ref(), ciphertext.as_ref(), nonce.as_ref()) {
        Ok(_) => {
            println!("Created Master Table");
            status=true;
        }
        Err(_) => {
            println!("Failed to create Master Table");
        }
    }

    status
}

fn create_db_submitted(input: SharedString, path: String) -> bool {
    create_db(input, path)
}

fn authenticate_submitted(input: SharedString, path: String) -> bool {
    on_authenticate(input, path)
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

fn get_initial_ui(db_exist: bool, path: String) -> Result<(), Box<dyn Error>> {
    let ui = EntryWindow::new()?;

    if db_exist {
        ui.set_current_page(Page::Authenticate);
    }
    else{
        ui.set_current_page(Page::CreateDb);
    }

    let ui_weak = ui.as_weak();

    // Clone for the first closure so originals remain available
    let path_for_create = path.clone();
    let ui_weak_for_create = ui_weak.clone();

    ui.on_create_db_submitted(move |input| {
        let is_created_db_success = create_db_submitted(input, path_for_create.clone());

        if is_created_db_success == false{
            println!("Database created Failed!");
            return;
        }
        if let Some(ui) = ui_weak_for_create.upgrade(){
            println!("Database created Successfully!");
            ui.set_current_page(Page::Authenticate);
        }
    });

    // Clone ui_weak again for the second closure (optional but clearer)
    let ui_weak_for_auth = ui_weak.clone();

    ui.on_authenticate_submitted(move |authenticate| {
        let is_authenticate_success = authenticate_submitted(authenticate, path.clone());

        if is_authenticate_success == false{
            println!("Authentication Failed!");
            return;
        }
        if let Some(ui) = ui_weak_for_auth.upgrade(){
            println!("Authentication Success");
            ui.set_current_page(Page::Passlock);
        }
    });

    ui.run()?;
    Ok(())
}
