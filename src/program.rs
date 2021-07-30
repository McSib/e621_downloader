use failure::Error;

use crate::e621::io::tag::{create_tag_file, parse_tag_file};
use crate::e621::io::{Config, Login};
use crate::e621::sender::RequestSender;
use crate::e621::WebConnector;
use console::Term;

pub struct Program {}

impl Program {
    pub fn new() -> Self {
        Program {}
    }

    pub fn run(&self) -> Result<(), Error> {
        Term::stdout().set_title("e621 downloader");
        trace!("Starting downloader...");

        // Check the config file and ensures that it is created.
        Config::check_config()?;

        // Create tag if it doesn't exist.
        create_tag_file()?;

        // Creates connector and requester to prepare for downloading posts.
        let login = Login::load().unwrap();
        let request_sender = RequestSender::new(&login);
        let mut connector = WebConnector::new(&request_sender);
        connector.should_enter_safe_mode();

        // Parses tag file.
        let groups = parse_tag_file(&request_sender)?;
        info!("Read tag file...");
        trace!("Parsed tag data, now attempting to obtain user blacklist...");

        // Collects all grabbed posts and moves it to connector to start downloading.
        if !login.is_empty() {
            connector.process_blacklist(&login.username);
            trace!("Parsed and processed user blacklist...")
        } else {
            trace!("Unable to obtain blacklist...")
        }

        connector.grab_all(&groups);
        connector.download_posts();

        info!("Finished downloading posts!");
        info!("Exiting...");

        Ok(())
    }
}
