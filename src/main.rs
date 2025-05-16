// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]



use dirs::home_dir;
use std::path::{Display, PathBuf};
use std::error::Error;
use std::fs;
use base64::Engine;
use slint::SharedString;



mod database;
mod encrypt;

slint::include_modules!();

const APP_NAME: &str = "RustPasswordManager";

// Assume you have a function to write to your database
fn on_authenticate(data: SharedString) -> Result<(), Box<dyn Error>> {
    println!("Simulating writing '{}' to the database...", data);
    // In a real application, you would have your database interaction logic here
    Ok(())
}


fn create_db(data: SharedString, path: String) -> Result<(), Box<dyn Error>> {
    //let Some(db_path) = get_user_db_path_cross_platform(db_file) else { todo!() };
    //print!("{}", db_path.to_str().unwrap());
    let manager = database::manager::DatabaseManager::new(path.as_str()).unwrap();
    println!("Creating DB with root cipher '{}' to the database...", data);
    manager.create_master_table();
    let salt = manager.get_salt();
    println!("Retrieved master salt... {:?}", salt);
    // In a real application, you would have your database interaction logic here
    encrypt::cryptography::CryptEngine::new(data.as_str(), &salt.unwrap()).expect("Encrypting data failed");
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
                match create_db(text_to_write, path.clone()) {
                    Ok(_) => println!("DB Created successfully."),
                    Err(e) => eprintln!("Error creating database: {}", e),
                }
            }
        });
        ui.run()?;
    }
    Ok(())
}

