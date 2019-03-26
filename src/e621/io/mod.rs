extern crate serde;

use serde::{Deserialize, Serialize};

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