extern crate chrono;

use std::error::Error;
use std::path::Path;

use chrono::{DateTime, Local};

use crate::e621::EWeb;
use crate::e621::io::{check_config, get_config, save_config};
use crate::e621::io::tag::{create_tag_file, parse_tag_file, TAG_NAME};

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Box<Error>> {
    // Check the config and load it.
    check_config()?;
    let mut config = get_config()?;

    // Create tag if it doesn't exist, then parse it.
    let tag_path = Path::new(TAG_NAME);
    create_tag_file(&tag_path)?;
    let tags = parse_tag_file(&tag_path)?;

    // Connect to e621, grab the posts, then download all of them.
    let mut connector = EWeb::new(&config);
    connector.get_posts(&tags)?;
    connector.download_posts()?;

    let date_time: DateTime<Local> = Local::now();
    config.last_run = date_time.format("%Y-%m-%d").to_string();
    save_config(&config)?;

    Ok(())
}
