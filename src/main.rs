#[macro_use]
extern crate failure;

use failure::Error;
use std::path::Path;

use crate::e621::io::tag::{create_tag_file, parse_tag_file, TAG_NAME};
use crate::e621::io::Config;
use crate::e621::EsixWebConnector;

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Error> {
    // Check the config and load it.
    Config::check_config()?;
    let mut config = Config::get_config()?;

    // Creates connector to prepare for downloading posts.
    let mut connector = EsixWebConnector::new(&mut config);
    connector.should_enter_safe_mode();

    // Create tag if it doesn't exist, then parse it.
    let tag_path = Path::new(TAG_NAME);
    create_tag_file(&tag_path)?;

    // Parse tag file
    let groups = parse_tag_file(&tag_path)?;
    println!("{:?}", groups);

    let collection = connector.grab_posts(&groups)?;
    connector.download_posts(collection)?;
    Config::save_config(&config)?;

    Ok(())
}
