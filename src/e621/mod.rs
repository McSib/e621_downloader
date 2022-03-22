use std::cell::RefCell;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use dialoguer::Confirm;
use failure::ResultExt;
use indicatif::ProgressBar;
use indicatif::{ProgressDrawTarget, ProgressStyle};

use blacklist::Blacklist;
use grabber::Grabber;
use io::tag::Group;
use io::Config;
use sender::RequestSender;

use crate::e621::sender::entries::UserEntry;

pub mod blacklist;
pub mod grabber;
pub mod io;
pub mod sender;

/// The `WebConnector` is the mother of all requests sent.
/// It manages how the API is called (through the `RequestSender`), how posts are grabbed (through calling its child `Grabber`), and how the posts are downloaded.
///
/// # Important
/// This is a large struct built on bringing the best performance possible without sacrificing any idiomatic code in the process.
/// When editing this struct, be sure that the changes you bring do not harm the overall performance, and if it does, be sure to give good reason on why the change is needed.
pub struct WebConnector {
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

impl WebConnector {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(request_sender: &RequestSender) -> Self {
        let config = Config::get_config().unwrap_or_default();
        WebConnector {
            request_sender: request_sender.clone(),
            download_directory: config.download_directory,
            progress_bar: ProgressBar::hidden(),
            grabber: Grabber::new(request_sender.clone()),
            blacklist: Rc::new(RefCell::new(Blacklist::new(request_sender.clone()))),
        }
    }

    /// Gets input and checks if the user wants to enter safe mode.
    /// If they do, the `RequestSender` will update the request urls for future sent requests.
    pub fn should_enter_safe_mode(&mut self) {
        trace!("Prompt for safe mode...");
        let confirm_prompt = Confirm::new()
            .with_prompt("Should enter safe mode?")
            .show_default(true)
            .default(false)
            .interact()
            .with_context(|e| {
                error!("Failed to setup confirmation prompt!");
                trace!("Terminal unable to set up confirmation prompt...");
                format!("{}", e)
            })
            .unwrap();

        trace!("Safe mode decision: {}", confirm_prompt);
        if confirm_prompt {
            self.request_sender.update_to_safe();
        }
    }

    /// Processes the blacklist and tokenizes for use when grabbing posts.
    pub fn process_blacklist(&mut self, username: &str) {
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

    /// Creates `Grabber` and grabs all posts before returning a tuple containing all general posts and single posts (posts grabbed by its ID).
    pub fn grab_all(&mut self, groups: &[Group]) {
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
                format!("{}", e)
            })
            .unwrap();
        trace!("Saved {}...", file_path);
    }

    /// Removes invalid characters from directory name.
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
        for collection in &self.grabber.posts {
            let short_collection_name = self.shorten_collection_name(&collection.name);
            let static_path: PathBuf = [
                &self.download_directory,
                &collection.category,
                &self.remove_invalid_chars(&collection.name),
            ]
            .iter()
            .collect();

            trace!("Printing Collection Info:");
            trace!("Collection Name:            \"{}\"", collection.name);
            trace!("Collection Category:        \"{}\"", collection.category);
            trace!("Collection Post Length:     \"{}\"", collection.posts.len());
            trace!(
                "Static file path for this collection: \"{}\"",
                static_path.to_str().unwrap()
            );

            for post in &collection.posts {
                self.progress_bar
                    .set_message(format!("Downloading: {} ", short_collection_name));
                let file_path: PathBuf = [
                    &static_path.to_str().unwrap().to_string(),
                    &self.remove_invalid_chars(&post.name),
                ]
                .iter()
                .collect();
                create_dir_all(file_path.parent().unwrap())
                    .with_context(|e| {
                        error!("Could not create directories for images!");
                        trace!("Directory path unable to be created...");
                        trace!(
                            "Path: \"{}\"",
                            file_path.parent().unwrap().to_str().unwrap()
                        );
                        format!("{}", e)
                    })
                    .unwrap();
                if file_path.exists() {
                    self.progress_bar
                        .set_message("Duplicate found: skipping... ");
                    self.progress_bar.inc(post.file_size as u64);
                    continue;
                }

                let bytes = self
                    .request_sender
                    .download_image(&post.url, post.file_size);
                self.save_image(file_path.to_str().unwrap(), &bytes);
                self.progress_bar.inc(post.file_size as u64);
            }

            trace!("Collection {} is finished downloading...", collection.name);
        }
    }

    /// Initializes the progress bar for downloading process.
    fn initialize_progress_bar(&mut self, len: u64) {
        self.progress_bar.set_length(len);
        self.progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} {eta}",
                )
                .unwrap()
                .progress_chars("=>-"),
        );
        self.progress_bar
            .set_draw_target(ProgressDrawTarget::stderr());
        self.progress_bar.reset();
        self.progress_bar
            .enable_steady_tick(Duration::from_millis(100));
    }

    /// Downloads tuple of general posts and single posts.
    pub fn download_posts(&mut self) {
        // Initializes the progress bar for downloading.
        let length = self.get_total_file_size();
        trace!("Total file size for all images grabbed is {}KB", length);
        self.initialize_progress_bar(length);
        self.download_collection();
        self.progress_bar.finish_and_clear();
    }

    /// Gets the total size (in KB) of every post image to be downloaded.
    fn get_total_file_size(&self) -> u64 {
        self.grabber
            .posts
            .iter()
            .map(|e| e.posts.iter().map(|f| f.file_size as u64).sum::<u64>())
            .sum()
    }
    fn shorten_collection_name(&self, name: &str) -> String {
        if name.len() >= 25 {
            let mut short_name = name[0..25].to_string();
            short_name.push_str("...");
            short_name
        } else {
            name.to_string()
        }
    }
}
