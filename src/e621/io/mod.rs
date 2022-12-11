/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use once_cell::sync::OnceCell;
use std::{
    fs::{read_to_string, write},
    io,
    path::Path,
    process::exit,
};

use failure::Error;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};

pub(crate) mod parser;
pub(crate) mod tag;

/// Name of the configuration file.
pub(crate) const CONFIG_NAME: &str = "config.json";

/// Name of the login file.
pub(crate) const LOGIN_NAME: &str = "login.json";

/// Config that is used to do general setup.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Config {
    /// The location of the download directory.
    #[serde(rename = "downloadDirectory")]
    download_directory: String,
    /// The file naming convention (e.g "md5", "id").
    #[serde(rename = "fileNamingConvention")]
    naming_convention: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    pub(crate) fn download_directory(&self) -> &str {
        &self.download_directory
    }

    pub(crate) fn naming_convention(&self) -> &str {
        &self.naming_convention
    }

    /// Checks config and ensure it isn't missing.
    pub(crate) fn config_exists() -> bool {
        if !Path::new(CONFIG_NAME).exists() {
            trace!("config.json: does not exist!");
            return false;
        }

        true
    }

    /// Creates config file.
    pub(crate) fn create_config() -> Result<(), Error> {
        let json = to_string_pretty(&Config::default())?;
        write(Path::new(CONFIG_NAME), json)?;

        Ok(())
    }

    /// Get the global instance of the `Config`.
    pub(crate) fn get() -> &'static Self {
        CONFIG.get_or_init(|| Self::get_config().unwrap())
    }

    /// Loads and returns `config` for quick management and settings.
    fn get_config() -> Result<Self, Error> {
        let mut config: Config = from_str(&read_to_string(CONFIG_NAME).unwrap())?;
        config.naming_convention = config.naming_convention.to_lowercase();
        let convention = ["md5", "id"];
        if !convention
            .iter()
            .any(|e| *e == config.naming_convention.as_str())
        {
            error!(
                "There is no naming convention {}!",
                config.naming_convention
            );
            info!("The naming convention can only be [\"md5\", \"id\"]");
            emergency_exit("Naming convention is incorrect!");
        }

        Ok(config)
    }
}

impl Default for Config {
    /// The default configuration for `Config`.
    fn default() -> Self {
        Config {
            download_directory: String::from("downloads/"),
            naming_convention: String::from("md5"),
        }
    }
}

/// `Login` contains all login information for obtaining information about a certain user.
/// This is currently only used for the blacklist.
#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Login {
    /// Username of user.
    #[serde(rename = "Username")]
    username: String,
    /// The password hash (also known as the API key) for the user.
    #[serde(rename = "APIKey")]
    api_key: String,
    /// Whether or not the user wishes to download their favorites.
    #[serde(rename = "DownloadFavorites")]
    download_favorites: bool,
}

static LOGIN: OnceCell<Login> = OnceCell::new();

impl Login {
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn api_key(&self) -> &str {
        &self.api_key
    }

    pub(crate) fn download_favorites(&self) -> bool {
        self.download_favorites
    }

    pub(crate) fn get() -> &'static Self {
        LOGIN.get_or_init(|| Self::load().unwrap_or_else(|e| {
            error!("Unable to load `login.json`. Error: {}", e);
            warn!("The program will use default values, but it is highly recommended to check your login.json file to \
			       ensure that everything is correct.");
            Login::default()
        }))
    }

    /// Loads the login file or creates one if it doesn't exist.
    fn load() -> Result<Self, Error> {
        let mut login = Login::default();
        let login_path = Path::new(LOGIN_NAME);
        if login_path.exists() {
            login = from_str(&read_to_string(login_path)?)?;
        } else {
            login.create_login()?;
        }

        Ok(login)
    }

    /// Checks if the login user and password is empty.
    pub(crate) fn is_empty(&self) -> bool {
        if self.username.is_empty() || self.api_key.is_empty() {
            return true;
        }

        false
    }

    /// Creates a new login file.
    fn create_login(&self) -> Result<(), Error> {
        write(LOGIN_NAME, to_string_pretty(self)?)?;

        info!("The login file was created.");
        info!(
            "If you wish to use your Blacklist, \
             be sure to give your username and API hash key."
        );
        info!(
            "Do not give out your API hash unless you trust this software completely, \
             always treat your API hash like your own password."
        );

        Ok(())
    }
}

impl Default for Login {
    /// The default state for the login if none exists.
    fn default() -> Self {
        Login {
            username: String::new(),
            api_key: String::new(),
            download_favorites: true,
        }
    }
}

/// Exits the program after message explaining the error and prompting the user to press `ENTER`.
pub(crate) fn emergency_exit(error: &str) {
    info!("{error}");
    println!("Press ENTER to close the application...");

    let mut line = String::new();
    io::stdin().read_line(&mut line).unwrap_or_default();

    exit(0x00FF);
}
