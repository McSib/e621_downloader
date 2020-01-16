#[macro_use]
extern crate failure;

use failure::Error;

use e621::io::tag::{create_tag_file, parse_tag_file};
use e621::io::Config;
use e621::sender::RequestSender;
use e621::WebConnector;

mod e621;

/// Main entry point of the application.
fn main() -> Result<(), Error> {
    // Check the config file and ensures that it is created.
    Config::check_config()?;

    // Create tag if it doesn't exist.
    create_tag_file()?;

    // Creates connector and requester to prepare for downloading posts.
    let request_sender = RequestSender::new();
    let mut connector = WebConnector::new(&request_sender);
    connector.should_enter_safe_mode();

    // Parses tag file.
    let groups = parse_tag_file(&request_sender)?;
    println!("Parsed tag file.");

    // Collects all grabbed posts and moves it to connector to start downloading.
    let grabbed_posts = connector.grab_posts(&groups);
    connector.download_grabbed_posts(grabbed_posts)?;

    Ok(())
}
