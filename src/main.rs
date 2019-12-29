#[macro_use]
extern crate failure;

use std::path::Path;

use failure::Error;

use crate::e621::io::Login;
use crate::e621::sender::RequestSender;
use e621::io::tag::{create_tag_file, parse_tag_file, TAG_NAME};
use e621::io::Config;
use e621::WebConnector;

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

    let request_sender = RequestSender::new();

    // Creates connector to prepare for downloading posts.
    let mut connector = WebConnector::new(&mut config, &login, request_sender.clone());
    connector.should_enter_safe_mode();
    connector.grab_blacklist()?;

    // Parse tag file
    let groups = parse_tag_file(&tag_path, request_sender.clone())?;
    println!("Parsed tag file.");

    // Collects all grabbed posts and moves it to connector to start downloading.
    let mut collection = connector.grab_posts(&groups)?;
    connector.download_posts_from_collection(&mut collection)?;

    // When posts are downloaded, save config with modified date.
    Config::save_config(&config)?;

    Ok(())
}
