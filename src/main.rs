// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::encrypt::cryptography::CryptEngine;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak, Timer, TimerMode};
use std::error::Error;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use zeroize::{Zeroize, Zeroizing};
use crate::database::manager::DatabaseManager;
use crate::session::session::Session;
use crate::sync::config::SyncConfig;
use crate::sync::restore::{self, ConflictChoice};
use crate::sync::SyncEngine;
use arboard::Clipboard;
use std::fs::File;
use std::io::BufReader;
use csv::{Writer, Reader};
use serde::{Serialize, Deserialize};
use rfd::FileDialog;
use log::{error, warn, info};

mod database;
mod encrypt;
mod session;
mod sync;

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
                    error!("Failed to decrypt master key");
                }
            };
            (status, session)
        }
        Err(e) => {
            error!("Failed to get master record: {}", e);
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
            return;
        }

        // Upgrade the weak UI pointer and change page
        if let Some(ui) = ui_weak.upgrade() {
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

    match manager.create_master_record(salt.as_ref(), ciphertext.as_ref(), nonce.as_ref()) {
        Ok(_) => {
            // Securely wipe sensitive data from memory now that it's committed to database
            salt.zeroize();
            master_key.zeroize();
            nonce.zeroize();
            ciphertext.zeroize();
            true
        }
        Err(e) => {
            error!("Failed to create Master Record: {}", e);
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
    
    // Basic validation for required fields (except password in edit mode)
    if service.is_empty() || email.is_empty() || username.is_empty() {
        return;
    }

    if let Some(ui) = ui_weak.upgrade() {
        if form_mode.as_str() == "Add" {
            // For Add mode, password is required
            if password.is_empty() {
                return;
            }
            insert_entry(session, &service, &email, username, password, notes, ui);
        } else {
            // For Edit mode, password can be empty (keep existing password)
            let record_id = record_id_str.as_str().parse::<i32>().unwrap_or(0);
            update_entry(session, index, record_id, &service, &email, username, password, notes, ui);
        }
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

    // If password is empty, retrieve the existing password from the database
    let password_to_use = if password.is_empty() {
        match session.get_decrypted_password(record_id) {
            Ok(existing_password) => SharedString::from(existing_password.as_str()),
            Err(e) => {
                error!("Failed to retrieve existing password: {}", e);
                return;
            }
        }
    } else {
        password
    };

    if session.update_entry(record_id, service, email, &username, &password_to_use, &notes) {
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
                error!("Failed to insert entry: {}", e);
            }
        }
    } else {
        error!("Could not access the table model as a VecModel");
    }
}

fn sort_table_handler(column: i32, ascending: bool, ui_weak: &Weak<EntryWindow>) {
    let ui = match ui_weak.upgrade() {
        Some(ui) => ui,
        None => return,
    };

    let table_model_handle = ui.global::<AppData>().get_table_rows();
    let model = match table_model_handle
        .as_any()
        .downcast_ref::<VecModel<ModelRc<StandardListViewItem>>>()
    {
        Some(model) => model,
        None => {
            error!("Could not access the table model as a VecModel");
            show_error_toast(ui_weak, "Sort failed: table model unavailable");
            return;
        }
    };

    let row_count = model.row_count();
    if row_count == 0 {
        return;
    }

    let column = column as usize;

    // Pull each row out of the model as a vector of cells, pairing it with a
    // single case-insensitive sort key for the clicked column. Rows shorter
    // than `column` sort under an empty key instead of panicking.
    let mut entries: Vec<(String, Vec<StandardListViewItem>)> = (0..row_count)
        .filter_map(|i| model.row_data(i))
        .map(|row| {
            let cell_count = row.row_count();
            let cells: Vec<StandardListViewItem> =
                (0..cell_count).filter_map(|j| row.row_data(j)).collect();
            let key = cells
                .get(column)
                .map(|cell| cell.text.as_str().to_lowercase())
                .unwrap_or_default();
            (key, cells)
        })
        .collect();

    if entries.is_empty() {
        return;
    }

    // Detect whether every value in the clicked column is numeric, so the
    // ID column sorts numerically (1, 2, 10, 11) instead of
    // lexicographically (1, 10, 11, ..., 2).
    let all_numeric = entries
        .iter()
        .all(|(key, _)| key.parse::<i64>().is_ok());

    // Stable sort by the precomputed key (one allocation per row, not per comparison)
    entries.sort_by(|a, b| {
        let ord = if all_numeric {
            let va = a.0.parse::<i64>().unwrap_or(0);
            let vb = b.0.parse::<i64>().unwrap_or(0);
            va.cmp(&vb)
        } else {
            a.0.cmp(&b.0)
        };
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });

    // Wipe the derived key material now that the order is fixed, so the
    // temporary lowercase copies do not linger in memory.
    for (key, _) in entries.iter_mut() {
        key.zeroize();
    }

    let new_model = Rc::new(VecModel::default());
    for (_, cells) in entries {
        new_model.push(ModelRc::new(VecModel::from(cells)));
    }

    ui.global::<AppData>().set_table_rows(ModelRc::from(new_model));
    info!("Sorted table by column {} ({})", column, if ascending { "ascending" } else { "descending" });
}

fn delete_entry(index: SharedString, session: &Session, ui_weak: Weak<EntryWindow>) {
    // Get the table model
    if let Some(_ui) = ui_weak.upgrade() {
        // Parse the index safely
        match index.as_str().parse::<i32>() {
            Ok(index_to_remove) => {
                if session.delete_entry(index_to_remove) {
                    refresh_table_data(&ui_weak, session);
                } else {
                    error!("Failed to delete entry from database");
                }
            }
            Err(e) => {
                error!("Failed to parse index '{}': {}", index, e);
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
            error!("Failed to check master table: {}", e);
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
                                    error!("Failed to write CSV record: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Failed to decrypt password for {}: {}", record.service, e);
                            }
                        }
                    }
                    
                    wtr.flush().ok();
                    
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
                    error!("Failed to create CSV file: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to get records for export: {}", e);
        }
    }
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
                            Err(e) => error!("Failed to import record: {}", e),
                        }
                    }
                    Err(e) => error!("Failed to parse CSV record: {}", e),
                }
            }
            
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
            error!("Failed to open CSV file: {}", e);
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
}

/// Apply the user's D4 conflict choice (docs/cloud-sync CS-4).
fn apply_conflict_choice(
    choice: ConflictChoice,
    ui_weak: &Weak<EntryWindow>,
    engine: &SyncEngine,
    session_state: &Arc<Mutex<Option<Session>>>,
    remote: &Arc<Mutex<Option<Vec<u8>>>>,
) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    // Lock the session first; only then take the remote bytes, so they
    // are never dropped without being resolved (D4: nothing is lost
    // silently). If the vault is busy the modal stays open and the
    // choice is not lost.
    let Ok(guard) = session_state.try_lock() else {
        ui.set_sync_status_detail(SharedString::from("Vault busy — try again in a moment."));
        return;
    };
    let Some(session) = guard.as_ref() else {
        ui.set_sync_show_conflict(false);
        return;
    };
    let bytes = match remote
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        Some(b) => b,
        None => {
            ui.set_sync_show_conflict(false);
            update_sync_status(ui_weak, engine, session_state);
            return;
        }
    };
    if let Err(e) = engine.resolve_conflict(session, choice, &bytes) {
        // Keep the modal open, put the remote bytes back, and report the
        // error so the user can retry or choose differently.
        *remote.lock().unwrap_or_else(|e| e.into_inner()) = Some(bytes);
        ui.set_sync_status_detail(SharedString::from(format!("Conflict resolution failed: {e}")));
        return;
    }
    if choice == ConflictChoice::Import {
        refresh_table_data(ui_weak, session);
    }
    ui.set_sync_show_conflict(false);
    update_sync_status(ui_weak, engine, session_state);
}

/// Push the current sync status onto the UI (docs/cloud-sync CS-4.2).
fn update_sync_status(
    ui_weak: &Weak<EntryWindow>,
    engine: &SyncEngine,
    session_state: &Arc<Mutex<Option<Session>>>,
) {
    let Some(ui) = ui_weak.upgrade() else {
        return;
    };
    let locked = session_state.try_lock().map(|g| g.is_none()).unwrap_or(true);
    let (state, detail) = engine.status(locked);
    ui.set_sync_status_state(SharedString::from(crate::sync::sync_state_word(state)));
    ui.set_sync_status_detail(SharedString::from(detail));
}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    info!("Starting Pandabox application");
    
    let (db_exist, manager) = init_manager();

    match manager {
        Some(manager) => run(db_exist, manager)?,
        None => error!("Failed to initialize database manager"),
    }
    Ok(())
}

fn run(db_exist: bool, manager: Rc<DatabaseManager>) -> Result<(), Box<dyn Error>> {
    get_initial_ui(db_exist, manager)?;
    Ok(())
}

fn get_initial_ui(db_exist: bool, manager: Rc<DatabaseManager>) -> Result<(), Box<dyn Error>> {
    let ui = EntryWindow::new()?;
    
    // Set version from Cargo.toml
    ui.set_app_version(SharedString::from(env!("CARGO_PKG_VERSION")));

    if db_exist {
        ui.set_current_page(Page::Authenticate);
    }
    else{
        ui.set_current_page(Page::CreateDb);
    }

    let ui_weak = ui.as_weak();

    // Clone before the callbacks take `manager` by move capture.
    let manager_for_sync = manager.clone();

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

    // Cloud sync (docs/cloud-sync CS-2): debounced writer, off by default.
    let sync_engine = Rc::new(SyncEngine::new());
    let sync_engine_for_auth = sync_engine.clone();
    let session_state_for_sync = Arc::clone(&session_state);

    ui.on_authenticate_submitted(move |input| {
        let ui_weak = ui_weak_for_auth.clone();
        let session_state = session_state_for_auth.clone();
        let (state, session) = authenticate_submitted(input, manager.clone());
        if state {
            if let Some(mut session) = session {
                // Arm cloud sync after every vault mutation (docs/cloud-sync D3).
                let engine = sync_engine_for_auth.clone();
                let state = session_state_for_sync.clone();
                session.set_on_vault_changed(Rc::new(move || {
                    engine.arm_with_timer(&state);
                }));

                // Update the session state
                *session_state.lock().unwrap() = Some(session);
                
                // Update UI
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_auth_error(false);
                    ui.set_current_page(Page::Passlock);
                    refresh_table_data(&ui_weak, session_state.lock().unwrap_or_else(|e| e.into_inner()).as_ref().unwrap());
                }
            }
        } else {
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
        }
    });

    let clipboard_holder: Arc<Mutex<Option<Clipboard>>> = Arc::new(Mutex::new(None));
    let clipboard_holder_for_copy = Arc::clone(&clipboard_holder);

    let ui_weak_for_clipboard = ui_weak.clone();
    let session_state_for_clipboard = Arc::clone(&session_state);
    ui.on_copy_to_clipboard(move |value: SharedString, field_name: SharedString| {
        let session_guard = session_state_for_clipboard.lock().unwrap();
        if let Some(session) = &*session_guard {
            copy_to_clipboard_handler(
                value,
                field_name,
                ui_weak_for_clipboard.clone(),
                session,
                Arc::clone(&clipboard_holder_for_copy),
            );
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

    let ui_weak_for_sort = ui_weak.clone();
    ui.on_sort_table(move |column: i32, ascending: bool| {
        sort_table_handler(column, ascending, &ui_weak_for_sort);
    });

    // --- Cloud sync UI (docs/cloud-sync CS-4) ---
    let sync_remote: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));

    let ui_w = ui_weak.clone();
    ui.on_choose_folder(move || {
        match rfd::FileDialog::new().pick_folder() {
            Some(path) => {
                let p = SharedString::from(path.to_string_lossy().into_owned());
                if let Some(ui) = ui_w.upgrade() {
                    ui.set_sync_folder(p.clone());
                }
                p
            }
            None => SharedString::default(),
        }
    });

    let engine = sync_engine.clone();
    let ui_w = ui_weak.clone();
    let state = session_state.clone();
    ui.on_set_sync_enabled(move |enabled| {
        let Some(ui) = ui_w.upgrade() else {
            return SharedString::default();
        };
        // Use the folder shown in the UI (the picker updates it), not the
        // folder from the persisted config.
        let result: Result<(), String> = if enabled {
            let folder = ui.get_sync_folder().as_str().to_owned();
            if folder.trim().is_empty() {
                Err("Choose a sync folder first".to_owned())
            } else {
                let folder_path = std::path::PathBuf::from(folder.trim());
                if !folder_path.is_dir() {
                    Err(format!(
                        "Sync folder is not an existing directory: {}",
                        folder.trim()
                    ))
                } else {
                    engine
                        .set_config(SyncConfig::Enabled { folder: folder_path })
                        .map_err(|e| e.to_string())
                }
            }
        } else {
            engine.set_config(SyncConfig::Disabled).map_err(|e| e.to_string())
        };
        match result {
            Ok(()) => {
                ui.set_sync_enabled(enabled);
                let msg = if enabled {
                    "Cloud sync enabled."
                } else {
                    "Cloud sync disabled. The synced file was left untouched."
                };
                update_sync_status(&ui_w, &engine, &state);
                SharedString::from(msg)
            }
            Err(msg) => {
                // Revert the checkbox so the UI and the config agree.
                ui.set_sync_enabled(!enabled);
                ui.set_sync_status_detail(SharedString::from(&msg));
                SharedString::from(msg)
            }
        }
    });

    let ui_w = ui_weak.clone();
    let engine = sync_engine.clone();
    let state = session_state.clone();
    let remote = sync_remote.clone();
    ui.on_sync_now(move || {
        let Ok(guard) = state.try_lock() else {
            update_sync_status(&ui_w, &engine, &state);
            return;
        };
        let Some(session) = guard.as_ref() else {
            update_sync_status(&ui_w, &engine, &state);
            return;
        };
        match engine.compare_with_remote(session) {
            restore::SyncComparison::InSync => {
                let _ = engine.flush(session);
                update_sync_status(&ui_w, &engine, &state);
            }
            restore::SyncComparison::Diverged { remote_bytes, remote_newer, .. } => {
                *remote
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(remote_bytes);
                if let Some(ui) = ui_w.upgrade() {
                    ui.set_sync_show_conflict(true);
                    ui.set_sync_conflict_detail(SharedString::from(if remote_newer {
                        "The file in the sync folder appears newer than your local vault. Import it, or keep your local vault — the other side is saved as a backup copy."
                    } else {
                        "Your local vault appears newer than the file in the sync folder. Keep your local vault, or import the file — the other side is saved as a backup copy."
                    }));
                }
            }
            other => {
                let mut flushed = false;
                let detail = match other {
                    restore::SyncComparison::NoFile => {
                        // No blob yet: "Sync now" writes the first one.
                        engine.mark_dirty();
                        match engine.flush(session) {
                            Ok(()) => {
                                flushed = true;
                                "Vault written to the sync folder.".to_owned()
                            }
                            Err(e) => format!("Sync failed: {e}"),
                        }
                    }
                    restore::SyncComparison::Undecodable => {
                        "The synced file could not be read with this vault's key.".to_owned()
                    }
                    restore::SyncComparison::Io(e) => e,
                    _ => String::new(),
                };
                if let Some(ui) = ui_w.upgrade() {
                    if !detail.is_empty() {
                        ui.set_sync_status_detail(SharedString::from(detail));
                    }
                }
                if flushed {
                    update_sync_status(&ui_w, &engine, &state);
                }
            }
        }
    });

    let ui_w = ui_weak.clone();
    let engine = sync_engine.clone();
    let state = session_state.clone();
    let remote = sync_remote.clone();
    ui.on_conflict_import(move || apply_conflict_choice(
        ConflictChoice::Import, &ui_w, &engine, &state, &remote,
    ));
    let ui_w = ui_weak.clone();
    let engine = sync_engine.clone();
    let state = session_state.clone();
    let remote = sync_remote.clone();
    ui.on_conflict_keep(move || apply_conflict_choice(
        ConflictChoice::KeepLocal, &ui_w, &engine, &state, &remote,
    ));
    let ui_w = ui_weak.clone();
    let engine = sync_engine.clone();
    let state = session_state.clone();
    let remote = sync_remote.clone();
    ui.on_conflict_cancel(move || apply_conflict_choice(
        ConflictChoice::Cancel, &ui_w, &engine, &state, &remote,
    ));

    let ui_w = ui_weak.clone();
    let state = session_state.clone();
    let manager_for_restore = manager_for_sync.clone();
    ui.on_restore_vault(move |folder: SharedString, password: SharedString| {
        let Some(ui) = ui_w.upgrade() else {
            return;
        };
        let mut folder = folder.as_str().trim().to_owned();
        if folder.is_empty() {
            folder = ui.get_sync_folder().as_str().trim().to_owned();
        }
        let folder_path = match restore::resolve_restore_folder(&folder) {
            Ok(path) => path,
            Err(e) => {
                ui.set_sync_status_detail(SharedString::from(e));
                return;
            }
        };
        if password.trim().is_empty() {
            ui.set_sync_status_detail(SharedString::from(
                "Enter the master password for that vault.",
            ));
            return;
        }
        // The password lives in a Zeroizing buffer for the duration of
        // the restore and is wiped when the closure body ends.
        let pw_bytes = Zeroizing::new(password.as_bytes().to_vec());
        let pw: &str = match std::str::from_utf8(&pw_bytes) {
            Ok(pw) => pw,
            Err(_) => {
                if let Some(ui) = ui_w.upgrade() {
                    ui.set_sync_status_detail(SharedString::from(
                        "Invalid password encoding; restore aborted.",
                    ));
                }
                return;
            }
        };
        match restore::restore_from_folder_with_password(&folder_path, pw, &manager_for_restore) {
            Ok(rv) => {
                let session = Session::new(rv.master_key.to_vec(), rv.engine, manager_for_restore.clone());
                *state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(session);
                if let Some(ui) = ui_w.upgrade() {
                    ui.set_sync_status_detail(SharedString::from("Vault restored."));
                    ui.set_current_page(Page::Passlock);
                }
                refresh_table_data(&ui_w, state.lock().unwrap_or_else(|e| e.into_inner()).as_ref().unwrap());
            }
            Err(e) => {
                if let Some(ui) = ui_w.upgrade() {
                    ui.set_sync_status_detail(SharedString::from(format!("Restore failed: {e}")));
                }
            }
        }
    });

    let ui_w = ui_weak.clone();
    let engine = sync_engine.clone();
    let state = session_state.clone();
    ui.on_sync_back(move || {
        if let Some(ui) = ui_w.upgrade() {
            ui.set_current_page(Page::Passlock);
        }
        update_sync_status(&ui_w, &engine, &state);
    });

    ui.run().unwrap();

    // Cloud sync (docs/cloud-sync D3): flush any pending change on clean
    // exit, then take the session out of the shared state so its
    // Zeroizing key buffer is wiped on drop.
    if let Some(session) = session_state.lock().unwrap().take() {
        if let Err(e) = sync_engine.flush(&session) {
            error!("Final sync flush failed: {}", e);
        }
    }

    Ok(())
}

fn copy_to_clipboard_handler(value: SharedString, field_name: SharedString, ui_weak: Weak<EntryWindow>, session: &Session, clipboard_holder: Arc<Mutex<Option<Clipboard>>>) {
    // Fetch and decrypt password if we're copying a password field
    let value_to_copy = if field_name.as_str() == "Password" {
        // Parse the record ID
        let record_id = match value.as_str().parse::<i32>() {
            Ok(id) => id,
            Err(e) => {
                error!("Failed to parse record ID: {}", e);
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
                error!("Failed to fetch/decrypt password: {}", e);
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

    // On Wayland (wayland-data-control feature): arboard uses the native wl-data-control protocol
    // and manages the clipboard lifetime internally via its own background thread.
    // On X11: the process must keep owning the selection until another owner takes it.
    // In both cases we store the Clipboard handle in clipboard_holder so it stays alive
    // until the next copy replaces it, preventing premature ownership loss.
    match Clipboard::new() {
        Ok(mut clipboard) => {
            match clipboard.set_text(&value_to_copy) {
                Ok(_) => {
                    info!("Copied {} to clipboard", field_name);
                    *clipboard_holder.lock().unwrap() = Some(clipboard);
                    
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
                    error!("Failed to copy to clipboard: {}", e);
                    show_error_toast(&ui_weak, "Failed to copy to clipboard");
                }
            }
        }
        Err(e) => {
            warn!("Failed to initialize clipboard (sandboxed environment): {}", e);
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
                error!("Failed to fetch records: {}", e);
            }
        }
    }
}
