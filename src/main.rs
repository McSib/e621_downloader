#[macro_use]
extern crate failure;

use std::path::Path;

use failure::Error;

use crate::e621::io::Login;
use e621::io::tag::{create_tag_file, parse_tag_file, TAG_NAME};
use e621::io::Config;
use e621::EsixWebConnector;

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Error> {
    // Check the config and load it.
    Config::check_config()?;
    let mut config = Config::get_config()?;

    // Loads login information for requests
    let login = Login::load()?;

    // Create tag if it doesn't exist, then parse it.
    let tag_path = Path::new(TAG_NAME);
    create_tag_file(&tag_path)?;

    // Creates connector to prepare for downloading posts.
    let mut connector = EsixWebConnector::new(&mut config, &login);
    connector.should_enter_safe_mode();
    connector.grab_blacklist()?;

    // Parse tag file
    let groups = parse_tag_file(&tag_path)?;
    println!("Parsed tag file.");

    // Collects all grabbed posts and moves it to connector to start downloading.
    let collection = connector.grab_posts(&groups)?;
    connector.download_posts_from_collection(collection)?;

    // When posts are downloaded, save config with modified date.
    Config::save_config(&config)?;

    Ok(())
}
