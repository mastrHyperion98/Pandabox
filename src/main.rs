// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;
use slint::SharedString;

slint::include_modules!();

// Assume you have a function to write to your database
fn write_data_to_database(data: SharedString) -> Result<(), Box<dyn Error>> {
    println!("Simulating writing '{}' to the database...", data);
    // In a real application, you would have your database interaction logic here
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    
    ui.on_write_to_db({
        move |text_to_write| {
            match write_data_to_database(text_to_write) {
                Ok(_) => println!("Data written successfully."),
                Err(e) => eprintln!("Error writing to database: {}", e),
            }
        }
    });

    ui.run()?;

    Ok(())
}
