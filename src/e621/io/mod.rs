extern crate failure;
extern crate serde;
extern crate serde_json;

use std::fs::{read_to_string, write};
use std::io;
use std::path::Path;
use std::process::exit;

use failure::Error;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty};

pub mod parser;
pub mod tag;

/// Name of the configuration file.
pub const CONFIG_NAME: &str = "config.json";
pub const LOGIN_NAME: &str = "login.json";

/// Config that is used to do general setup.
#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
	/// The location of the download directory
	#[serde(rename = "downloadDirectory")]
	pub download_directory: String,
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
	fn create_config() -> Result<(), Error> {
		let json = to_string_pretty(&Config::default())?;
		write(Path::new(CONFIG_NAME), json)?;

		Ok(())
	}

	/// Checks if config exist and, if not, creates config template.
	pub fn check_config() -> Result<(), Error> {
		if !Config::config_exists() {
			println!("Creating config...");
			Config::create_config()?;
		}

		Ok(())
	}

	/// Loads and returns `config` for quick management and settings.
	pub fn get_config() -> Result<Config, Error> {
		let config: Config = from_str(&read_to_string(CONFIG_NAME).unwrap())?;
		Ok(config)
	}
}

impl Default for Config {
	/// The default configuration for `Config`.
	fn default() -> Self {
		Config {
			download_directory: String::from("downloads/"),
		}
	}
}

/// `Login` contains all login information for obtaining information about a certain user.
/// This is currently only used for the blacklist.
#[derive(Serialize, Deserialize, Clone)]
pub struct Login {
	/// Username of user.
	#[serde(rename = "Username")]
	pub username: String,
	/// The password hash (also known as the API key) for the user.
	#[serde(rename = "APIKey")]
	pub api_key: String,
	/// Whether or not the user wishes to download their favorites.
	#[serde(rename = "DownloadFavorites")]
	pub download_favorites: bool,
}

impl Login {
	/// Loads the login file or creates one if it doesn't exist.
	pub fn load() -> Result<Self, Error> {
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
	pub fn is_empty(&self) -> bool {
		if self.username.is_empty() || self.api_key.is_empty() {
			return true;
		}

		false
	}

	/// Creates a new login file.
	fn create_login(&self) -> Result<(), Error> {
		write(LOGIN_NAME, to_string_pretty(self)?)?;

		println!("The login file was created.");
		println!(
			"If you wish to use your Blacklist, \
             be sure to give your username and API hash key."
		);
		println!(
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
pub fn emergency_exit(error: &str) {
	println!("{}", error);
	println!("Press ENTER to close the application...");

	let mut line = String::new();
	io::stdin().read_line(&mut line).unwrap_or_default();

	exit(0x00FF);
}
