// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::encrypt::cryptography::CryptEngine;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak, Timer, TimerMode};
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;
use crate::database::manager::DatabaseManager;
use crate::session::session::Session;
use arboard::Clipboard;
use std::fs::File;
use std::io::BufReader;
use csv::{Writer, Reader};
use serde::{Serialize, Deserialize};
use rfd::FileDialog;

mod database;
mod encrypt;
mod session;

slint::include_modules!();

const APP_NAME: &str = "Pandabox";

#[derive(Debug, Serialize, Deserialize)]
struct CsvRecord {
    service: String,
    email: String,
    username: String,
    password: String,
    notes: String,
}

// Assume you have a function to write to your database
fn on_authenticate(
    data: SharedString,
    manager: Rc<DatabaseManager>
) -> (bool, Option<Session>) {
    match manager.get_master_record() {
        Ok(mut master_record) => {
            let engine = CryptEngine::new(data.as_str(), &master_record.salt).unwrap();
            let mut status = false;
            let mut session = None;
            match engine.decrypt_master_key(&master_record.nonce, &master_record.encrypted_master_key) {
                Ok(decrypted_key) => {
                    master_record.encrypted_master_key.zeroize();
                    master_record.nonce.zeroize();
                    master_record.salt.zeroize();
                    session = Some(Session::new(decrypted_key, engine.clone(), manager.clone()));
                    status = true;
                }
                Err(_) => {
                    println!("Failed to decrypt master key");
                }
            };
            (status, session)
        }
        Err(e) => {
            eprintln!("Failed to get master record: {}", e);
            (false, None)
        }
    }
}

fn make_db_callback<F, T>(
    manager: Rc<DatabaseManager>,
    ui_weak: Weak<EntryWindow>,
    fn_to_call: F,
    success_page: Page,
) -> impl Fn(T) + 'static
where
    F: Fn(T, Rc<DatabaseManager>) -> bool + Send + Sync + 'static,
    T: Clone + 'static,
{
    let manager = manager.clone(); // captured by the closure
    let ui_weak = ui_weak.clone();

    move |input: T| {
        // Call the supplied function
        if !fn_to_call(input.clone(), manager.clone()) {
            println!("{:?} failed!", stringify!(fn_to_call));
            return;
        }

        // Upgrade the weak UI pointer and change page
        if let Some(ui) = ui_weak.upgrade() {
            println!("{:?} succeeded!", stringify!(fn_to_call));
            ui.set_current_page(success_page);
        }
    }
}


fn create_db(manager: Rc<DatabaseManager>, data: SharedString) -> bool {
    // Before creating the database perhaps we should create the salt, nonce and encyrption key
    let mut salt = CryptEngine::generate_salt();
    let engine = CryptEngine::new(data.as_str(), &salt).unwrap();
    let mut master_key = CryptEngine::generate_master_key();
    let (mut nonce, mut ciphertext) = engine.encrypt_master_key(master_key.as_ref()).unwrap();

    println!("Creating DB");
    match manager.create_master_record(salt.as_ref(), ciphertext.as_ref(), nonce.as_ref()) {
        Ok(_) => {
            println!("Created Master Record");
            // Securely wipe sensitive data from memory now that it's committed to database
            salt.zeroize();
            master_key.zeroize();
            nonce.zeroize();
            ciphertext.zeroize();
            true
        }
        Err(e) => {
            eprintln!("Failed to create Master Record: {}", e);
            // Securely wipe sensitive data from memory
            salt.zeroize();
            master_key.zeroize();
            false
        }
    }
}

fn handle_save_service(
    session: &Session,
    form_mode: SharedString,
    current_index: i32,
    record_id_str: SharedString,
    service: SharedString,
    email: SharedString,
    username: SharedString,
    password: SharedString,
    notes: SharedString,
    ui_weak: slint::Weak<EntryWindow>
) {
    let index = current_index as usize;
    if service.is_empty() || email.is_empty() || username.is_empty() || password.is_empty(){
        return;
    }

    if let Some(ui) = ui_weak.upgrade() {
        println!("Form mode: {}", form_mode);
        println!("Current index: {}", current_index);
        if form_mode.as_str() == "Add" {
            insert_entry(session, &service, &email, username, password, notes, ui);
        }else{
            let record_id = record_id_str.as_str().parse::<i32>().unwrap_or(0);
            update_entry(session, index, record_id, &service, &email, username, password, notes, ui);
        }

        println!("Service saved: {} - {}", service, email);
    }
}

fn update_entry(session: &Session, index: usize, record_id: i32, service: &SharedString, email: &SharedString, username: SharedString, password: SharedString, notes: SharedString, ui: EntryWindow) {
    let table_model_handle = ui.global::<AppData>().get_table_rows();
    let row_data: ModelRc<StandardListViewItem> = ModelRc::new(VecModel::from(vec![
        StandardListViewItem::from(record_id.to_string().as_str()),
        StandardListViewItem::from(service.as_str()),
        StandardListViewItem::from(email.as_str()),
        StandardListViewItem::from(username.as_str()),
        StandardListViewItem::from("••••••••"), // Hide password in display
        StandardListViewItem::from(notes.as_str()),
    ]));

    if session.update_entry(record_id, service, email, &username, &password, &notes) {
        table_model_handle.set_row_data(index, row_data);
    }
}

fn insert_entry(session: &Session, service: &SharedString, email: &SharedString, username: SharedString, password: SharedString, notes: SharedString, ui: EntryWindow) {
    // Get current timestamp
    let table_model_handle = ui.global::<AppData>().get_table_rows();
    // This is the key: We "downcast" the generic model handle to the specific
    // type we know it is: a VecModel that holds rows.
    // This "unlocks" the .push() method.
    if let Some(vec_model) = table_model_handle.as_any().downcast_ref::<VecModel<ModelRc<StandardListViewItem>>>() {

        // Insert the entry and get the created record with its ID
        match session.insert_entry(service, email, &username, &password, &notes) {
            Ok(record) => {
                // Create a new row with the actual ID from the database
                let row_data: Vec<slint::StandardListViewItem> = vec![
                    StandardListViewItem::from(record.id.to_string().as_str()),
                    StandardListViewItem::from(record.service.as_str()),
                    StandardListViewItem::from(record.email.as_str()),
                    StandardListViewItem::from(record.username.as_str()),
                    StandardListViewItem::from("••••••••"), // Hide password in display
                    StandardListViewItem::from(record.notes.as_str()),
                ];

                let new_row = ModelRc::new(VecModel::from(row_data));
                vec_model.push(new_row);
            }
            Err(e) => {
                eprintln!("Failed to insert entry: {}", e);
            }
        }
    } else {
        // This will print an error to your console if the type isn't what we expect.
        println!("Error: Could not access the table model as a VecModel.");
    }
}

fn delete_entry(index: SharedString, session: &Session, ui_weak: Weak<EntryWindow>) {
    // Get the table model
    if let Some(_ui) = ui_weak.upgrade() {
        // Parse the index safely
        match index.as_str().parse::<i32>() {
            Ok(index_to_remove) => {
                println!("Removing entry at index: {}", index_to_remove);
                if session.delete_entry(index_to_remove) {
                    refresh_table_data(&ui_weak, session);
                } else {
                    eprintln!("Failed to delete entry from database");
                }
            }
            Err(e) => {
                eprintln!("Failed to parse index '{}': {}", index, e);
            }
        }
    }
}

fn create_db_submitted(input: SharedString, database_manager: Rc<DatabaseManager>) -> bool {
    create_db(database_manager, input)
}

fn authenticate_submitted(input: SharedString, database_manager: Rc<DatabaseManager>) -> (bool, Option<Session>) {
    on_authenticate(input, database_manager)
}

fn init_manager() -> (bool, Option<Rc<DatabaseManager>>) {
    // The .env file is loaded by the DatabaseManager constructor, so we just need to create it.
    let manager = Rc::new(DatabaseManager::new());

    // Check if the master table has any records.
    // This tells us if the database has been initialized.
    match manager.check_master_table_exists() {
        Ok(exists) => (exists, Some(manager)),
        Err(e) => {
            eprintln!("Failed to check master table: {}", e);
            // This might happen if the database file is corrupt or migrations haven't run.
            // For now, we'll treat it as if the DB doesn't exist.
            (false, Some(manager))
        }
    }
}

fn export_csv_handler(session: &Session, ui_weak: Weak<EntryWindow>) {
    // Open file dialog to select save location
    let file_path = FileDialog::new()
        .set_file_name("pandabox_export.csv")
        .add_filter("CSV Files", &["csv"])
        .add_filter("All Files", &["*"])
        .save_file();
    
    if let Some(path) = file_path {
        // Get all records
        match session.get_all_records() {
            Ok(records) => {
            match File::create(&path) {
                Ok(file) => {
                    let mut wtr = Writer::from_writer(file);
                    
                    // Write each record with decrypted password
                    for record in records {
                        match session.decrypt_password(&record.password) {
                            Ok(decrypted_password) => {
                                let csv_record = CsvRecord {
                                    service: record.service,
                                    email: record.email,
                                    username: record.username,
                                    password: decrypted_password,
                                    notes: record.notes,
                                };
                                
                                if let Err(e) = wtr.serialize(csv_record) {
                                    eprintln!("Failed to write record: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to decrypt password for {}: {}", record.service, e);
                            }
                        }
                    }
                    
                    wtr.flush().ok();
                    println!("Exported to: {}", path.display());
                    
                    // Show success toast
                    if let Some(ui) = ui_weak.upgrade() {
                        let msg = format!("Exported to {}", path.file_name().unwrap_or_default().to_string_lossy());
                        ui.global::<AppData>().set_toast_message(SharedString::from(msg));
                        ui.global::<AppData>().set_show_toast(true);
                        
                        let ui_weak_timer = ui.as_weak();
                        let timer = Timer::default();
                        timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(3), move || {
                            if let Some(ui) = ui_weak_timer.upgrade() {
                                ui.global::<AppData>().set_show_toast(false);
                            }
                        });
                        std::mem::forget(timer);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create CSV file: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to get records: {}", e);
        }
    }
    } else {
        println!("Export cancelled by user");
    }
}

fn import_csv_handler(session: &Session, ui_weak: Weak<EntryWindow>) {
    // Open file dialog to select CSV file
    let file_path = FileDialog::new()
        .add_filter("CSV Files", &["csv"])
        .add_filter("All Files", &["*"])
        .pick_file();
    
    if let Some(path) = file_path {
    match File::open(&path) {
        Ok(file) => {
            let mut rdr = Reader::from_reader(BufReader::new(file));
            let mut count = 0;
            
            for result in rdr.deserialize() {
                match result {
                    Ok(record) => {
                        let csv_record: CsvRecord = record;
                        match session.insert_entry(
                            &SharedString::from(csv_record.service),
                            &SharedString::from(csv_record.email),
                            &SharedString::from(csv_record.username),
                            &SharedString::from(csv_record.password),
                            &SharedString::from(csv_record.notes),
                        ) {
                            Ok(_) => count += 1,
                            Err(e) => eprintln!("Failed to import record: {}", e),
                        }
                    }
                    Err(e) => eprintln!("Failed to parse CSV record: {}", e),
                }
            }
            
            println!("Imported {} records", count);
            
            // Refresh table
            refresh_table_data(&ui_weak, session);
            
            // Show success toast
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<AppData>().set_toast_message(SharedString::from(format!("Imported {} records", count)));
                ui.global::<AppData>().set_show_toast(true);
                
                let ui_weak_timer = ui.as_weak();
                let timer = Timer::default();
                timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(3), move || {
                    if let Some(ui) = ui_weak_timer.upgrade() {
                        ui.global::<AppData>().set_show_toast(false);
                    }
                });
                std::mem::forget(timer);
            }
        }
        Err(e) => {
            eprintln!("Failed to open CSV file: {}", e);
            if let Some(ui) = ui_weak.upgrade() {
                ui.global::<AppData>().set_toast_message(SharedString::from("Failed to open CSV file"));
                ui.global::<AppData>().set_show_toast(true);
                
                let ui_weak_timer = ui.as_weak();
                let timer = Timer::default();
                timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(3), move || {
                    if let Some(ui) = ui_weak_timer.upgrade() {
                        ui.global::<AppData>().set_show_toast(false);
                    }
                });
                std::mem::forget(timer);
            }
        }
    }
    } else {
        println!("Import cancelled by user");
    }
}

fn save_all_handler(ui_weak: Weak<EntryWindow>) {
    // SQLite auto-commits, but we can show a confirmation
    if let Some(ui) = ui_weak.upgrade() {
        ui.global::<AppData>().set_toast_message(SharedString::from("All data saved!"));
        ui.global::<AppData>().set_show_toast(true);
        
        let ui_weak_timer = ui.as_weak();
        let timer = Timer::default();
        timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(2), move || {
            if let Some(ui) = ui_weak_timer.upgrade() {
                ui.global::<AppData>().set_show_toast(false);
            }
        });
        std::mem::forget(timer);
    }
    println!("Database saved");
}

fn main() -> Result<(), Box<dyn Error>> {
    let (db_exist, manager) = init_manager();

    match manager {
        Some(manager) => run(db_exist, manager)?,
        None => eprintln!("Failed to initialize database manager"),
    }
    Ok(())
}

fn run(db_exist: bool, manager: Rc<DatabaseManager>) -> Result<(), Box<dyn Error>> {
    get_initial_ui(db_exist, manager)?;
    Ok(())
}

fn get_initial_ui(db_exist: bool, manager: Rc<DatabaseManager>) -> Result<(), Box<dyn Error>> {
    let ui = EntryWindow::new()?;

    if db_exist {
        ui.set_current_page(Page::Authenticate);
    }
    else{
        ui.set_current_page(Page::CreateDb);
    }

    let ui_weak = ui.as_weak();

    ui.on_create_db_submitted(make_db_callback(
        manager.clone(),
        ui_weak.clone(),
        create_db_submitted,
        Page::Authenticate,
    ));

    // Create a session state that will be shared between the UI and the authentication callback
    let session_state = Arc::new(Mutex::new(None::<Session>));
    let session_state_for_auth = Arc::clone(&session_state);
    let ui_weak_for_auth = ui_weak.clone();
    
    ui.on_authenticate_submitted(move |input| {
        let ui_weak = ui_weak_for_auth.clone();
        let session_state = session_state_for_auth.clone();
        let (state, session) = authenticate_submitted(input, manager.clone());
        if state {
            if let Some(session) = session {
                // Update the session state
                *session_state.lock().unwrap() = Some(session);
                
                // Update UI
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_auth_error(false);
                    ui.set_current_page(Page::Passlock);
                    refresh_table_data(&ui_weak, session_state.lock().unwrap().as_ref().unwrap());
                }
            }
        } else {
            println!("Failed to authenticate");
            // Trigger shake animation
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_auth_error(true);
                
                // Trigger shake sequence
                let ui_weak_shake = ui.as_weak();
                std::thread::spawn(move || {
                    for i in 1..=6 {
                        std::thread::sleep(std::time::Duration::from_millis(80));
                        let ui_weak_clone = ui_weak_shake.clone();
                        slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_weak_clone.upgrade() {
                                ui.set_shake_trigger(i);
                            }
                        }).ok();
                    }
                    // Reset shake
                    std::thread::sleep(std::time::Duration::from_millis(80));
                    let ui_weak_clone = ui_weak_shake.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_clone.upgrade() {
                            ui.set_shake_trigger(0);
                        }
                    }).ok();
                    
                    // Clear error after 2 seconds
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    let ui_weak_clone = ui_weak_shake.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak_clone.upgrade() {
                            ui.set_auth_error(false);
                        }
                    }).ok();
                });
            }
        }
    });
    ui.on_generate_password(|| SharedString::from(CryptEngine::generate_random_password()));

    let ui_weak_for_save = ui_weak.clone();
    let session_state_for_save = Arc::clone(&session_state);
    ui.on_save_service(move |data: ServiceData, mode: SharedString, row: i32| {
        let session_guard = session_state_for_save.lock().unwrap();
        if let Some(session) = &*session_guard {
            // Extract the service data from the ServiceData struct
            handle_save_service(
                session, 
                mode, 
                row,
                data.id, 
                data.service, 
                data.email, 
                data.username, 
                data.password, 
                data.notes, 
                ui_weak_for_save.clone()
            );
        } else {
            println!("No active session found");
        }
    });

    let ui_weak_for_delete = ui_weak.clone();
    let session_state_for_delete = Arc::clone(&session_state);
    ui.on_delete_entry(move |index: SharedString | {
        let session_guard = session_state_for_delete.lock().unwrap();
        if let Some(session) = &*session_guard {
            // Extract the service data from the ServiceData struct
            delete_entry(
                index,
                session,
                ui_weak_for_delete.clone()
            );
        } else {
            println!("No active session found");
        }
    });

    let ui_weak_for_clipboard = ui_weak.clone();
    let session_state_for_clipboard = Arc::clone(&session_state);
    ui.on_copy_to_clipboard(move |value: SharedString, field_name: SharedString| {
        let session_guard = session_state_for_clipboard.lock().unwrap();
        if let Some(session) = &*session_guard {
            copy_to_clipboard_handler(value, field_name, ui_weak_for_clipboard.clone(), session);
        } else {
            println!("No active session found for clipboard operation");
        }
    });

    let ui_weak_for_export = ui_weak.clone();
    let session_state_for_export = Arc::clone(&session_state);
    ui.on_export_csv(move || {
        let session_guard = session_state_for_export.lock().unwrap();
        if let Some(session) = &*session_guard {
            export_csv_handler(session, ui_weak_for_export.clone());
        }
    });

    let ui_weak_for_import = ui_weak.clone();
    let session_state_for_import = Arc::clone(&session_state);
    ui.on_import_csv(move || {
        let session_guard = session_state_for_import.lock().unwrap();
        if let Some(session) = &*session_guard {
            import_csv_handler(session, ui_weak_for_import.clone());
        }
    });

    let ui_weak_for_save = ui_weak.clone();
    ui.on_save_all(move || {
        save_all_handler(ui_weak_for_save.clone());
    });

    ui.run().unwrap();

    Ok(())
}

fn copy_to_clipboard_handler(value: SharedString, field_name: SharedString, ui_weak: Weak<EntryWindow>, session: &Session) {
    // Fetch and decrypt password if we're copying a password field
    let value_to_copy = if field_name.as_str() == "Password" {
        // Parse the record ID
        let record_id = match value.as_str().parse::<i32>() {
            Ok(id) => id,
            Err(e) => {
                eprintln!("Failed to parse record ID: {}", e);
                if let Some(ui) = ui_weak.upgrade() {
                    ui.global::<AppData>().set_toast_message(SharedString::from("Invalid record ID"));
                    ui.global::<AppData>().set_show_toast(true);
                    
                    let ui_weak_timer = ui.as_weak();
                    slint::invoke_from_event_loop(move || {
                        let timer = Timer::default();
                        timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(2), move || {
                            if let Some(ui) = ui_weak_timer.upgrade() {
                                ui.global::<AppData>().set_show_toast(false);
                            }
                        });
                        std::mem::forget(timer);
                    }).ok();
                }
                return;
            }
        };

        // Fetch from database and decrypt
        match session.get_decrypted_password(record_id) {
            Ok(decrypted) => decrypted,
            Err(e) => {
                eprintln!("Failed to fetch/decrypt password: {}", e);
                // Show error toast
                if let Some(ui) = ui_weak.upgrade() {
                    ui.global::<AppData>().set_toast_message(SharedString::from("Failed to decrypt password"));
                    ui.global::<AppData>().set_show_toast(true);
                    
                    let ui_weak_timer = ui.as_weak();
                    slint::invoke_from_event_loop(move || {
                        let timer = Timer::default();
                        timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(2), move || {
                            if let Some(ui) = ui_weak_timer.upgrade() {
                                ui.global::<AppData>().set_show_toast(false);
                            }
                        });
                        std::mem::forget(timer);
                    }).ok();
                }
                return;
            }
        }
    } else {
        value.to_string()
    };

    // Copy to clipboard with retry logic for Flatpak environments
    let mut clipboard_result = Clipboard::new();
    
    // If first attempt fails, try again after a short delay (helps with X11 timeout issues)
    if clipboard_result.is_err() {
        std::thread::sleep(std::time::Duration::from_millis(50));
        clipboard_result = Clipboard::new();
    }
    
    match clipboard_result {
        Ok(mut clipboard) => {
            match clipboard.set_text(&value_to_copy) {
                Ok(_) => {
                    println!("Copied {} to clipboard", field_name);
                    
                    // Keep clipboard alive for a bit to ensure clipboard managers can read it
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        drop(clipboard);
                    });
                    
                    // Show toast notification
                    if let Some(ui) = ui_weak.upgrade() {
                        let message = format!("{} copied!", field_name);
                        
                        // Set the toast message and show it
                        ui.global::<AppData>().set_toast_message(SharedString::from(message));
                        ui.global::<AppData>().set_show_toast(true);
                        
                        // Create a timer to hide the toast after 2 seconds
                        let ui_weak_timer = ui.as_weak();
                        slint::invoke_from_event_loop(move || {
                            let timer = Timer::default();
                            timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(2), move || {
                                if let Some(ui) = ui_weak_timer.upgrade() {
                                    ui.global::<AppData>().set_show_toast(false);
                                }
                            });
                            // Keep timer alive by leaking it (it will auto-cleanup after firing)
                            std::mem::forget(timer);
                        }).ok();
                    }
                }
                Err(e) => {
                    eprintln!("Failed to copy to clipboard: {}", e);
                    show_error_toast(&ui_weak, "Failed to copy to clipboard");
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize clipboard: {}", e);
            show_error_toast(&ui_weak, "Clipboard unavailable in sandbox");
        }
    }
}

fn show_error_toast(ui_weak: &Weak<EntryWindow>, message: &str) {
    if let Some(ui) = ui_weak.upgrade() {
        ui.global::<AppData>().set_toast_message(SharedString::from(message));
        ui.global::<AppData>().set_show_toast(true);
        
        let ui_weak_timer = ui.as_weak();
        slint::invoke_from_event_loop(move || {
            let timer = Timer::default();
            timer.start(TimerMode::SingleShot, std::time::Duration::from_secs(3), move || {
                if let Some(ui) = ui_weak_timer.upgrade() {
                    ui.global::<AppData>().set_show_toast(false);
                }
            });
            std::mem::forget(timer);
        }).ok();
    }
}

fn refresh_table_data(ui_weak: &Weak<EntryWindow>, session: &Session) {
    if let Some(ui) = ui_weak.upgrade() {
        println!("Refreshing table data...");
        
        // Get all records through the session
        match session.get_all_records() {
            Ok(records) => {
                println!("Retrieved {} records from database", records.len());
                
                // Create a new model for the table
                let table_model = Rc::new(VecModel::default());
                
                // Add each record to the model
                for record in records {
                    let row = vec![
                        StandardListViewItem::from(record.id.to_string().as_str()),
                        StandardListViewItem::from(record.service.as_str()),
                        StandardListViewItem::from(record.email.as_str()),
                        StandardListViewItem::from(record.username.as_str()),
                        StandardListViewItem::from("••••••••"), // Hide password in display
                        StandardListViewItem::from(record.notes.as_str()),
                    ];
                    
                    let row_model = Rc::new(VecModel::from(row));
                    table_model.push(row_model.into());
                }
                
                // Update the UI with the new model
                ui.global::<AppData>().set_table_rows(ModelRc::from(table_model));
            }
            Err(e) => {
                eprintln!("Failed to fetch records: {}", e);
            }
        }
    }
}
