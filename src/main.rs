// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::encrypt::cryptography::CryptEngine;
use dirs::home_dir;
use slint::{Model, ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
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
    // Now we create the database
    let (mut key, mut nonce, mut salt) = manager.get_master_record().unwrap();

    let engine = CryptEngine::new(data.as_str(), &salt).unwrap();
    let mut status = false;
    let mut session = None;
    match engine.decrypt_master_key(nonce.as_slice(), key.as_ref()) {
        Ok(decrypted_key) => {
            println!("Master key successfully decrypted");
            //data.zeroize();
            key.zeroize();
            nonce.zeroize();
            salt.zeroize();
            session = Some(Session::new(decrypted_key, engine.clone()));
            status=true;
        }
        Err(_) => {
            println!("Failed to decrypt master key");
        }
    };

    (status, session)

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
    let mut status = false;
    match manager.create_master_table(salt.as_ref(), ciphertext.as_ref(), nonce.as_ref()) {
        Ok(_) => {
            println!("Created Master Table");
            // Securely wipe sensitive data from memory now that it's committed to database
            salt.zeroize();
            master_key.zeroize();
            nonce.zeroize();
            ciphertext .zeroize();
            match manager.create_record_table() {
                Ok(_) => {
                    println!("Created Record Table");
                    status=true;
                }
                Err(_) => {
                    println!("Failed to create Record Table");
                }
            };
        }
        Err(_) => {
            // Securely wipe sensitive data from memory
            salt.zeroize();
            master_key.zeroize();
            println!("Failed to create Master Table");
        }
    }
    status
}

fn handle_save_service(
    form_mode: SharedString,
    current_index: i32,
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
            insert_entry(&service, &email, username, password, notes, ui);
        }else{
            update_entry(index, &service, &email, username, password, notes, ui);
        }

        println!("Service saved: {} - {}", service, email);
    }
}

fn update_entry(index: usize, service: &SharedString, email: &SharedString, username: SharedString, password: SharedString, notes: SharedString, ui: EntryWindow) {
    let table_model_handle = ui.global::<AppData>().get_table_rows();
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    let row_data: ModelRc<StandardListViewItem> = ModelRc::new(VecModel::from(vec![
        StandardListViewItem::from(service.as_str()),
        StandardListViewItem::from(email.as_str()),
        StandardListViewItem::from(username.as_str()),
        StandardListViewItem::from(password.as_str()),
        StandardListViewItem::from(notes.as_str()),
        StandardListViewItem::from(now.as_str()),
    ]));

    // TODO: UPDATE DATABASE
    // TODO: IF SUCCESS SET ROW DATA VALUE
    table_model_handle.set_row_data(index, row_data);
}

fn insert_entry(service: &SharedString, email: &SharedString, username: SharedString, password: SharedString, notes: SharedString, ui: EntryWindow) {
    // Get current timestamp
    let table_model_handle = ui.global::<AppData>().get_table_rows();
    // This is the key: We "downcast" the generic model handle to the specific
    // type we know it is: a VecModel that holds rows.
    // This "unlocks" the .push() method.
    if let Some(vec_model) = table_model_handle.as_any().downcast_ref::<VecModel<ModelRc<StandardListViewItem>>>() {
        // --- Your code to create the new row is perfect ---
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let row_data: Vec<slint::StandardListViewItem> = vec![
            StandardListViewItem::from(service.as_str()),
            StandardListViewItem::from(email.as_str()),
            StandardListViewItem::from(username.as_str()),
            StandardListViewItem::from(password.as_str()),
            StandardListViewItem::from(notes.as_str()),
            StandardListViewItem::from(now.as_str()),
        ];

        // TODO: WRITE TO DATABASE
        // TODO: IF SUCCESS ADD TO I
        let new_row = ModelRc::new(VecModel::from(row_data));
        // Now that we have the concrete `vec_model`, we can push the new row directly.
        // The UI will update automatically.
        vec_model.push(new_row);
    } else {
        // This will print an error to your console if the type isn't what we expect.
        println!("Error: Could not access the table model as a VecModel.");
    }
}

fn delete_entry(index: usize, ui: EntryWindow) {
    // TODO: DELETE FROM DATABASE
    // TODO: IF SUCCESS REMOVE FROM UI
    let table_model_handle = ui.global::<AppData>().get_table_rows();
    if let Some(vec_model) = table_model_handle.as_any().downcast_ref::<VecModel<ModelRc<StandardListViewItem>>>() {
        vec_model.remove(index);
    }
}

fn create_db_submitted(input: SharedString, database_manager: Rc<DatabaseManager>) -> bool {
    create_db(database_manager, input)
}

fn authenticate_submitted(input: SharedString, database_manager: Rc<DatabaseManager>) -> (bool, Option<Session>) {
    on_authenticate(input, database_manager)
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

fn init_manager() -> (bool, Option<Rc<DatabaseManager>>) {
    let db_file = "db.sqlite";

    if let Some(db_path) = get_user_db_path_cross_platform(db_file) {
        println!("Potential database path: {}", db_path.display());
        if let Some(parent_dir) = db_path.parent() {
            if !parent_dir.exists() {
                println!("Directory {:?} does not exist. Creating...", parent_dir);
                fs::create_dir_all(parent_dir).expect("TODO: panic message");
            }
        }
        let path_str = db_path.to_str().unwrap_or_default().to_owned();
        let manager = Rc::new(DatabaseManager::new(path_str.as_str()).unwrap());

        println!("Database file exists at: {}", db_path.display());
        (manager.check_master_table().is_ok(), Some(manager))
    } else {
        println!("Could not determine the user's home directory.");
        (false, None)
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

    let ui_weak_on_authenticate_submitted = ui.as_weak();
    ui.on_authenticate_submitted(move |input| {
        let (state, session) = authenticate_submitted(input, manager.clone());
        if state {
            ui_weak_on_authenticate_submitted.upgrade().unwrap().set_current_page(Page::Passlock);
        }
    });


    ui.on_generate_password(|| SharedString::from(CryptEngine::generate_random_password()));

    let ui_weak_for_save = ui_weak.clone();
    ui.on_save_service(move |data, form, index| {
        handle_save_service(
            form,
            index,
            data.service,
            data.email,
            data.username,
            data.password,
            data.notes,
            ui_weak_for_save.clone()
        );
    });

    ui.run()?;
    Ok(())
}
