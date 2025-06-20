use crate::{create_db, on_authenticate, AuthenticateWindow, CreateDbWindow};
use slint::ComponentHandle;

pub fn setupt_createdb_handler(ui_weak_handle: slint::Weak<CreateDbWindow>, data_path: String) {
    ui_weak_handle.unwrap().on_createdb({
        move |text_to_write| {
            let ui_handle = ui_weak_handle.unwrap();

            match create_db(text_to_write, data_path.clone(), ui_handle.window()) {
                Ok(_) => println!("DB Created successfully."),
                Err(e) => eprintln!("Error creating database: {}", e),
            }
        }
    });
}

pub fn setup_authentication_handler(
    ui_weak_handle: slint::Weak<AuthenticateWindow>,
    data_path: String,
) {
    ui_weak_handle
        .unwrap()
        .on_authenticate(move |text_to_write| {
            // We get a strong handle inside the closure each time the event fires
            let ui_handle = ui_weak_handle.unwrap();

            match on_authenticate(text_to_write, data_path.clone(), ui_handle.window()) {
                Ok(_) => {
                    println!("Authentication Success");
                    ui_handle.set_is_error(false); // Reset error state on success
                    ui_handle.set_error_msg("".into()); // Clear error message
                }
                Err(e) => {
                    ui_handle.set_is_error(true);
                    ui_handle.set_error_msg("Incorrect Password. Failed to Authenticate".into());
                    eprintln!("Authentication Failed: {}", e);
                }
            }
        });
}
