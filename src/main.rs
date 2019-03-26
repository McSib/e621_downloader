extern crate chrono;
extern crate serde_json;

use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, Local};
use serde_json::to_string_pretty;

use crate::e621::io::{Config, CONFIG_NAME};

mod e621;

/// Checks config and ensure it isn't missing.
fn check_config() -> bool {
    if !Path::new(CONFIG_NAME).exists() {
        println!("config.json: does not exist!");
        return false;
    }

    true
}

/// Creates config if it does not exist.
fn create_config() -> Result<(), Box<Error>> {
    let date_time: DateTime<Local> = Local::now();
    let json = to_string_pretty(&Config {
        create_directories: true,
        download_directory: String::from("download/"),
        last_run: date_time.format("%Y/%m/%d").to_string(),
        part_used_as_name: String::from("md5"),
    })?;

    let mut config = File::create(Path::new(CONFIG_NAME))?;
    config.write(&json.as_bytes())?;

    Ok(())
}

/// Main entry point of the application.
fn main() -> Result<(), Box<Error>> {
    let config_exists = check_config();
    if !config_exists {
        create_config()?;
    }

    let config = serde_json::from_str::<Config>(&read_to_string(Path::new(CONFIG_NAME)).unwrap())?;

    Ok(())
}
