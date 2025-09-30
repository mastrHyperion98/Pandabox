// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::encrypt::cryptography::CryptEngine;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use zeroize::Zeroize;
use crate::database::manager::DatabaseManager;
use crate::session::session::Session;

mod database;
mod encrypt;
mod session;

slint::include_modules!();

const APP_NAME: &str = "Pandabox";

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
        StandardListViewItem::from("••••••••"), // Hide password
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
                    StandardListViewItem::from("••••••••"), // Hide password
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
    if let Some(ui) = ui_weak.upgrade() {
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
                    ui.set_current_page(Page::Passlock);
                    refresh_table_data(&ui_weak, session_state.lock().unwrap().as_ref().unwrap());
                }
            }
        } else {
            println!("Failed to authenticate");
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

    ui.run().unwrap();

    Ok(())
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
                        StandardListViewItem::from("••••••••"), // Hide password
                        StandardListViewItem::from(record.notes.as_str()),
                        StandardListViewItem::from(""), // Placeholder for last update
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
