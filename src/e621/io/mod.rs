extern crate serde;
extern crate serde_json;

use std::collections::HashMap;
use std::error::Error;
use std::fs::{read_to_string, write};
use std::io;
use std::path::Path;
use std::process::exit;

use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};

pub mod tag;

/// Name of the configuration file.
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
    /// Holds all dates for every tag used.
    #[serde(rename = "lastRun")]
    pub last_run: HashMap<String, String>,
    /// Which part should be used as the name, that of which are: "id", or "md5"
    #[serde(rename = "partUsedAsName")]
    pub part_used_as_name: String,
}

impl Config {
    /// Checks config and ensure it isn't missing.
    fn config_exists() -> bool {
        if !Path::new(CONFIG_NAME).exists() {
            println!("config.json: does not exist!");
            return false;
        }

        true
    }

    /// Creates config file.
    fn create_config() -> Result<(), Box<Error>> {
        let json = to_string_pretty(&Config::default())?;
        write(Path::new(CONFIG_NAME), json)?;

        Ok(())
    }

    /// Checks if config exist and, if not, creates config template.
    pub fn check_config() -> Result<(), Box<Error>> {
        if !Config::config_exists() {
            println!("Creating config...");
            return Config::create_config();
        }

        Ok(())
    }

    /// Loads and returns `config` for quick management and settings.
    pub fn get_config() -> Result<Config, Box<Error>> {
        let config = from_str::<Config>(&read_to_string(Path::new(CONFIG_NAME)).unwrap())?;
        Ok(config)
    }

    /// Saves new configuration for future run.
    pub fn save_config(config: &Config) -> Result<(), Box<Error>> {
        let json = serde_json::to_string_pretty(config)?;
        write(Path::new(CONFIG_NAME), json)?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            create_directories: true,
            download_directory: String::from("downloads/"),
            last_run: HashMap::new(),
            part_used_as_name: String::from("md5"),
        }
    }
}

/// Exits the program after message explaining the error and prompting the user to press `ENTER`.
pub fn emergency_exit(error: &str) {
    println!("{}", error);
    println!("Press ENTER to close the application...");

    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap_or_default();

    exit(0);
}
