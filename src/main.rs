extern crate chrono;
extern crate serde_json;

use std::error::Error;
use std::fs::{read_to_string};
use std::path::Path;

use crate::e621::io::{Config, CONFIG_NAME, check_config, create_config};
use crate::e621::web::{EWeb, Post};

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Box<Error>> {
    let config_exists = check_config();
    if !config_exists {
        create_config()?;
    }

    let config = serde_json::from_str::<Config>(&read_to_string(Path::new(CONFIG_NAME)).unwrap())?;
    let mut connector = EWeb::new(&config);
    connector.add_tags(&vec!["score:>=70"]);
    let posts = connector.get_posts()?;
    for post in &posts {
        println!("{}", post.id);
    }

    Ok(())
}
