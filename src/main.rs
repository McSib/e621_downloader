extern crate chrono;
extern crate serde_json;

use std::error::Error;
use std::fs::{read_to_string};
use std::path::Path;

use crate::e621::io::{Config, CONFIG_NAME, check_config, get_config};
use crate::e621::web::{EWeb, Post};

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Box<Error>> {
    check_config();

    let config = get_config()?;
    let mut connector = EWeb::new(&config);
    connector.add_tags(&vec!["score:>=70"]);
    let posts = connector.get_posts()?;

    Ok(())
}
