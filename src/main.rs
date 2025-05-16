// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]



use dirs::home_dir;
use std::path::PathBuf;
use std::error::Error;
use argon2::{Argon2, Params, PasswordHasher, Version};
use argon2::password_hash::SaltString;
use base64::Engine;
use base64::engine::{general_purpose, GeneralPurpose};
use slint::SharedString;
use crate::database::manager;



mod database;

slint::include_modules!();

const APP_NAME: &str = "RustPasswordManager";

// Assume you have a function to write to your database
fn on_authenticate(data: SharedString) -> Result<(), Box<dyn Error>> {
    println!("Simulating writing '{}' to the database...", data);
    // In a real application, you would have your database interaction logic here
    Ok(())
}

pub fn derive_key(password: &str, salt: &[u8]) -> Result<(), Box<dyn Error>>  {
    // 1. Configure Argon2 parameters for key derivation
    let argon2 = Argon2::default();
    let salt_string = SaltString::encode_b64(&salt).expect("Failed to encode salt");


    let password_hash = argon2.hash_password(password.as_bytes(), &salt_string).expect("Failed to hash password");

    println!("Password hash is {}", password_hash);

    Ok(()) // Return the derived key
}

fn create_db(data: SharedString) -> Result<(), Box<dyn Error>> {
    let db_file = "db.sqlite";
    //let Some(db_path) = get_user_db_path_cross_platform(db_file) else { todo!() };
    //print!("{}", db_path.to_str().unwrap());
    let manager = database::manager::DatabaseManager::new( db_file).unwrap();
    println!("Creating DB with root cipher '{}' to the database...", data);
    manager.create_master_table();
    let salt = manager.get_salt();
    println!("Retrieved master salt... {:?}", salt);
    let key = derive_key(data.as_str(), &salt.unwrap());
    // In a real application, you would have your database interaction logic here
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

fn check_db_exist() -> bool{
    let db_file = "db.sqlite";
    if let Some(db_path) = get_user_db_path_cross_platform(db_file) {
        println!("Potential database path: {}", db_path.display());
        if db_path.exists() {
            println!("Database file exists at: {}", db_path.display());
            true
        } else {
            println!("Database file does not exist at: {}", db_path.display());
            false
        }
    } else {
        println!("Could not determine the user's home directory.");
        false
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let db_exist = check_db_exist();
    if db_exist {
        let ui = AuthenticateWindow::new()?;
        ui.on_authenticate({
            move |text_to_write| {
                match on_authenticate(text_to_write) {
                    Ok(_) => println!("Data written successfully."),
                    Err(e) => eprintln!("Error writing to database: {}", e),
                }
            }
        });

        ui.run()?;
    } 
    else {
        let ui = CreateDbWindow::new()?;
        ui.on_createdb({
            move |text_to_write| {
                match create_db(text_to_write) {
                    Ok(_) => println!("DB Created successfully."),
                    Err(e) => eprintln!("Error creating database: {}", e),
                }
            }
        });
        ui.run()?;
    }
    Ok(())
}

