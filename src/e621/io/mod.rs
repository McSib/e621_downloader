extern crate serde;

use serde::{Deserialize, Serialize};
use std::path::Path;
use chrono::{DateTime, Local};
use serde_json::to_string_pretty;
use std::fs::File;
use std::error::Error;
use std::io::Write;

pub static CONFIG_NAME: &'static str = "config.json";

/// Config that is used to do general setup.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    /// Whether or not to create a directory for every tag used to search for images
    #[serde(rename = "createDirectories")]
    pub create_directories: bool,
    /// The location of the download directory
    #[serde(rename = "downloadDirectory")]
    pub download_directory: String,
    /// The last time the program was ran
    #[serde(rename = "lastRun")]
    pub last_run: String,
    /// Which part should be used as the name, that of which are: "id", or "md5"
    #[serde(rename = "partUsedAsName")]
    pub part_used_as_name: String,
}

/// Checks config and ensure it isn't missing.
///
/// # Example
///
/// ```
/// let config_exists = check_config();
/// ```
pub fn check_config() -> bool {
    if !Path::new(CONFIG_NAME).exists() {
        println!("config.json: does not exist!");
        return false;
    }

    true
}

/// Creates config file.
///
/// # Example
///
/// ```
/// let config_exists = check_config();
/// if !config_exists {
///     create_config();
/// }
/// ```
pub fn create_config() -> Result<(), Box<Error>> {
    let date_time: DateTime<Local> = Local::now();
    let json = to_string_pretty(&Config {
        create_directories: true,
        download_directory: String::from("download/"),
        last_run: date_time.format("%Y-%m-%d").to_string(),
        part_used_as_name: String::from("md5"),
    })?;

    let mut config = File::create(Path::new(CONFIG_NAME))?;
    config.write(&json.as_bytes())?;

    Ok(())
}