#[macro_use]
extern crate failure;

use std::error::Error;
use std::path::Path;

use crate::e621::io::tag::{create_tag_file, parse_tag_file, TAG_NAME};
use crate::e621::io::Config;
use crate::e621::EsixWebConnector;

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Box<Error>> {
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
    let tags = parse_tag_file(&tag_path)?;
    println!("{:?}", tags);

    connector.grab_posts(&tags)?;

    // TODO: Delete all the commented code before releasing 0.7.0.

    //    connector.download_posts();
    //    connector.get_posts(&tags);
    //    println!("{:#?}", connector.post_collection);

    // Validate tags and group names
    //    let mut validator = TagValidator::new();
    //    validator.validate_and_identify_tags(&mut tags);
    //    println!("{:?}", tags);
    //    if validator.validate_groups() {
    //        // Connect to e621, grab the posts, then download all of them.
    //        let mut connector = EsixWebConnector::new(&mut config);
    //        connector.check_for_safe_mode()?;
    //        connector.get_posts(&groups)?;
    //        connector.download_posts()?;
    //
    //        // Update the date for future runs.
    //        save_config(&config)?;
    //    }

    Ok(())
}
