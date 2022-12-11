/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::{env::current_dir, fs::write, path::Path};

use console::Term;
use failure::Error;

use crate::e621::{
    io::{
        emergency_exit,
        tag::{parse_tag_file, TAG_FILE_EXAMPLE, TAG_NAME},
        Config, Login,
    },
    sender::RequestSender,
    E621WebConnector,
};

/// The name of the cargo package.
const NAME: &str = env!("CARGO_PKG_NAME");

/// The version of the cargo package.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The authors who created the package.
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

pub(crate) struct Program;

impl Program {
    pub(crate) fn new() -> Self {
        Self
    }

    pub(crate) fn run(&self) -> Result<(), Error> {
        Term::stdout().set_title("e621 downloader");
        trace!("Starting e621 downloader...");
        trace!("Program Name: {}", NAME);
        trace!("Program Version: {}", VERSION);
        trace!("Program Authors: {}", AUTHORS);
        trace!(
            "Program Working Directory: {}",
            current_dir()
                .expect("Unable to get working directory!")
                .to_str()
                .unwrap()
        );

        // Check the config file and ensures that it is created.
        trace!("Checking if config file exists...");
        if !Config::config_exists() {
            trace!("Config file doesn't exist...");
            info!("Creating config file...");
            Config::create_config()?;
        }

        // Create tag if it doesn't exist.
        trace!("Checking if tag file exists...");
        if !Path::new(TAG_NAME).exists() {
            info!("Tag file does not exist, creating tag file...");
            write(TAG_NAME, TAG_FILE_EXAMPLE)?;
            trace!("Tag file \"{}\" created...", TAG_NAME);

            emergency_exit(
                "The tag file is created, the application will close so you can include \
             the artists, sets, pools, and individual posts you wish to download.",
            );
        }

        // Creates connector and requester to prepare for downloading posts.
        let login = Login::get();
        trace!("Login information loaded...");
        trace!("Login Username: {}", login.username());
        trace!("Login API Key: {}", "*".repeat(login.api_key().len()));
        trace!("Login Download Favorites: {}", login.download_favorites());

        let request_sender = RequestSender::new(login);
        let mut connector = E621WebConnector::new(&request_sender);
        connector.should_enter_safe_mode();

        // Parses tag file.
        trace!("Parsing tag file...");
        let groups = parse_tag_file(&request_sender)?;

        // Collects all grabbed posts and moves it to connector to start downloading.
        if !login.is_empty() {
            trace!("Parsing user blacklist...");
            connector.process_blacklist(login.username());
        } else {
            trace!("Skipping blacklist as user is not logged in...");
        }

        connector.grab_all(&groups);
        connector.download_posts();

        info!("Finished downloading posts!");
        info!("Exiting...");

        Ok(())
    }
}
