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

use std::cell::RefCell;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use dialoguer::Confirm;
use failure::ResultExt;
use indicatif::{ProgressBar, ProgressDrawTarget};

use crate::e621::blacklist::Blacklist;
use crate::e621::grabber::{Grabber, Shorten};
use crate::e621::io::{Config, Login};
use crate::e621::io::tag::Group;
use crate::e621::sender::entries::UserEntry;
use crate::e621::sender::RequestSender;
use crate::e621::tui::{ProgressBarBuilder, ProgressStyleBuilder};

pub(crate) mod blacklist;
pub(crate) mod grabber;
pub(crate) mod io;
pub(crate) mod sender;
pub(crate) mod tui;

/// A web connector that manages how the API is called (through the [RequestSender]), how posts are grabbed
/// (through [Grabber]), and how the posts are downloaded.
pub(crate) struct E621WebConnector {
    /// The sender used for all API calls.
    request_sender: RequestSender,
    /// The config which is modified when grabbing posts.
    download_directory: String,
    /// Progress bar that displays the current progress in downloading posts.
    progress_bar: ProgressBar,
    /// Grabber which is responsible for grabbing posts.
    grabber: Grabber,
    /// The user's blacklist.
    blacklist: Rc<RefCell<Blacklist>>,
}

impl E621WebConnector {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub(crate) fn new(request_sender: &RequestSender) -> Self {
        E621WebConnector {
            request_sender: request_sender.clone(),
            download_directory: Config::get().download_directory().to_string(),
            progress_bar: ProgressBar::hidden(),
            grabber: Grabber::new(request_sender.clone(), false),
            blacklist: Rc::new(RefCell::new(Blacklist::new(request_sender.clone()))),
        }
    }

    /// Gets input and enters safe depending on user choice.
    pub(crate) fn should_enter_safe_mode(&mut self) {
        trace!("Prompt for safe mode...");
        let confirm_prompt = Confirm::new()
            .with_prompt("Should enter safe mode?")
            .show_default(true)
            .default(false)
            .interact()
            .with_context(|e| {
                error!("Failed to setup confirmation prompt!");
                trace!("Terminal unable to set up confirmation prompt...");
                format!("{e}")
            })
            .unwrap();

        trace!("Safe mode decision: {confirm_prompt}");
        if confirm_prompt {
            self.request_sender.update_to_safe();
            self.grabber.set_safe_mode(true);
        }
    }

    /// Processes the blacklist and tokenizes for use when grabbing posts.
    pub(crate) fn process_blacklist(&mut self) {
        let username = Login::get().username();
        let user: UserEntry = self
            .request_sender
            .get_entry_from_appended_id(username, "user");
        if let Some(blacklist_tags) = user.blacklisted_tags {
            if !blacklist_tags.is_empty() {
                let blacklist = self.blacklist.clone();
                blacklist
                    .borrow_mut()
                    .parse_blacklist(blacklist_tags)
                    .cache_users();
                self.grabber.set_blacklist(blacklist);
            }
        }
    }

    /// Creates `Grabber` and grabs all posts before returning a tuple containing all general posts and single posts
    /// (posts grabbed by its ID).
    ///
    /// # Arguments
    ///
    /// * `groups`: The groups to grab from.
    pub(crate) fn grab_all(&mut self, groups: &[Group]) {
        trace!("Grabbing posts...");
        self.grabber.grab_favorites();
        self.grabber.grab_posts_by_tags(groups);
    }

    /// Saves image to download directory.
    fn save_image(&self, file_path: &str, bytes: &[u8]) {
        write(file_path, bytes)
            .with_context(|e| {
                error!("Failed to save image!");
                trace!("A downloaded image was unable to be saved...");
                format!("{e}")
            })
            .unwrap();
        trace!("Saved {file_path}...");
    }

    /// Removes invalid characters from directory path.
    ///
    /// # Arguments
    ///
    /// * `dir_name`: Directory name to remove invalid chars from.
    ///
    /// returns: String
    fn remove_invalid_chars(&self, dir_name: &str) -> String {
        dir_name
            .chars()
            .map(|e| match e {
                '?' | ':' | '*' | '<' | '>' | '\"' | '|' => '_',
                _ => e,
            })
            .collect()
    }

    /// Processes `PostSet` and downloads all posts from it.
    fn download_collection(&mut self) {
        for collection in self.grabber.posts().iter() {
            let collection_name = collection.name();
            let collection_category = collection.category();
            let collection_posts = collection.posts();
            let collection_count = collection_posts.len();
            let short_collection_name = collection.shorten("...");

            #[cfg(unix)]
            let static_path: PathBuf = [
                &self.download_directory,
                collection.category(),
                &self.remove_invalid_chars(collection_name),
            ]
            .iter()
            .collect();

            #[cfg(windows)]
            let mut static_path: PathBuf = [
                &self.download_directory,
                collection.category(),
                &self.remove_invalid_chars(collection_name),
            ]
            .iter()
            .collect();

            // This is put here to attempt to shorten the length of the path if it passes window's
            // max path length.
            #[cfg(windows)]
            const MAX_PATH: usize = 260; // Defined in Windows documentation.

            #[cfg(windows)]
            let start_path_len = static_path.as_os_str().len();

            #[cfg(windows)]
            if start_path_len >= MAX_PATH {
                static_path = [
                    &self.download_directory,
                    collection_category,
                    &self.remove_invalid_chars(&collection.shorten('_')),
                ]
                .iter()
                .collect();

                let new_len = static_path.as_os_str().len();
                if new_len >= MAX_PATH {
                    error!("Path is too long and crosses the {MAX_PATH} char limit.\
                       Please relocate the program to a directory closer to the root drive directory.");
                    trace!("Path length: {new_len}");
                }
            }

            trace!("Printing Collection Info:");
            trace!("Collection Name:            \"{collection_name}\"");
            trace!("Collection Category:        \"{collection_category}\"");
            trace!("Collection Post Length:     \"{collection_count}\"");
            trace!(
                "Static file path for this collection: \"{}\"",
                static_path.to_str().unwrap()
            );

            for post in collection_posts {
                let file_path: PathBuf = [
                    &static_path.to_str().unwrap().to_string(),
                    &self.remove_invalid_chars(post.name()),
                ]
                .iter()
                .collect();

                if file_path.exists() {
                    self.progress_bar
                        .set_message("Duplicate found: skipping... ");
                    self.progress_bar.inc(post.file_size() as u64);
                    continue;
                }

                self.progress_bar
                    .set_message(format!("Downloading: {short_collection_name} "));

                let parent_path = file_path.parent().unwrap();
                create_dir_all(parent_path)
                    .with_context(|e| {
                        error!("Could not create directories for images!");
                        trace!("Directory path unable to be created...");
                        trace!("Path: \"{}\"", parent_path.to_str().unwrap());
                        format!("{e}")
                    })
                    .unwrap();

                let bytes = self
                    .request_sender
                    .download_image(post.url(), post.file_size());
                self.save_image(file_path.to_str().unwrap(), &bytes);
                self.progress_bar.inc(post.file_size() as u64);
            }

            trace!("Collection {collection_name} is finished downloading...");
        }
    }

    /// Initializes the progress bar for downloading process.
    ///
    /// # Arguments
    ///
    /// * `len`: The total bytes to download.
    fn initialize_progress_bar(&mut self, len: u64) {
        self.progress_bar = ProgressBarBuilder::new(len)
            .style(
                ProgressStyleBuilder::default()
                    .template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {binary_bytes_per_sec} {eta}")
                    .progress_chars("=>-")
                    .build())
            .draw_target(ProgressDrawTarget::stderr())
            .reset()
            .steady_tick(Duration::from_secs(1))
            .build();
    }

    /// Downloads tuple of general posts and single posts.
    pub(crate) fn download_posts(&mut self) {
        // Initializes the progress bar for downloading.
        let length = self.get_total_file_size();
        trace!("Total file size for all images grabbed is {length}KB");
        self.initialize_progress_bar(length);
        self.download_collection();
        self.progress_bar.finish_and_clear();
    }

    /// Gets the total size (in KB) of every post image to be downloaded.
    fn get_total_file_size(&self) -> u64 {
        self.grabber
            .posts()
            .iter()
            .map(|e| e.posts().iter().map(|f| f.file_size() as u64).sum::<u64>())
            .sum()
    }
}
