extern crate dialoguer;
extern crate failure;
extern crate indicatif;

use std::fs::create_dir_all;
use std::path::PathBuf;

use dialoguer::Confirmation;
use failure::Error;
use indicatif::ProgressBar;

use io::Config;
use io::tag::Group;

use crate::e621::grabber::{Grabber, PostSet};
use crate::e621::sender::RequestSender;

use self::indicatif::{ProgressDrawTarget, ProgressStyle};

pub mod blacklist;
pub mod grabber;
pub mod io;
pub mod sender;

fn get_length_posts(sets: &[PostSet], single_set: &PostSet) -> u64 {
    let mut total_size = 0;
    for set in sets {
        for post in &set.posts {
            total_size += post.file_size as u64;
        }
    }

    for post in &single_set.posts {
        total_size += post.file_size as u64;
    }

    total_size
}

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
}

impl WebConnector {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(request_sender: &RequestSender) -> Self {
        WebConnector {
            request_sender: request_sender.clone(),
            download_directory: Config::get_config().unwrap_or_default().download_directory,
            progress_bar: ProgressBar::hidden(),
        }
    }

    /// Gets input and checks if the user wants to enter safe mode.
    /// If they do, the `RequestSender` will update the request urls for future sent requests.
    pub fn should_enter_safe_mode(&mut self) {
        if Confirmation::new()
            .with_text("Should enter safe mode?")
            .show_default(true)
            .interact()
            .unwrap_or_default()
        {
            self.request_sender.update_to_safe();
        }
    }

    /// Creates `Grabber` and grabs all posts before returning a tuple containing all general posts and single posts (posts grabbed by its ID).
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<(Vec<PostSet>, PostSet), Error> {
        let grabber = Grabber::from_tags(groups, self.request_sender.clone())?;
        Ok((grabber.grabbed_posts, grabber.grabbed_single_posts))
    }

    /// Saves image to download directory.
    fn save_image(&mut self, file_path: &str, bytes: &[u8]) -> Result<(), Error> {
        std::fs::write(file_path, bytes)?;
        Ok(())
    }

    /// Removes invalid characters from directory name.
    fn remove_invalid_chars(&self, dir_name: &mut String) {
        for character in &["?", ":", "*", "<", ">", "\"", "|"] {
            *dir_name = dir_name.replace(character, "_");
        }
    }

    /// Processes `PostSet` and downloads all posts from it.
    fn download_set(&mut self, set: &mut PostSet) -> Result<(), Error> {
        self.remove_invalid_chars(&mut set.set_name);
        for post in &set.posts {
            self.progress_bar
                .set_message(format!("Downloading: {} ", set.set_name).as_str());
            let file_path: PathBuf = [
                &self.download_directory,
                &set.category,
                &set.set_name,
                &post.file_name,
            ]
                .iter()
                .collect();
            create_dir_all(&file_path)?;
            if file_path.exists() {
                self.progress_bar
                    .set_message("Duplicate found: skipping... ");
                continue;
            }

            let bytes = self
                .request_sender
                .download_image(&post.file_url, post.file_size)?;
            self.save_image(file_path.to_str().unwrap(), &bytes)?;
            self.progress_bar.inc(post.file_size as u64);
        }

        Ok(())
    }

    /// Downloads all posts from an array of sets
    fn download_sets(&mut self, sets: &mut [PostSet]) -> Result<(), Error> {
        for set in sets {
            self.download_set(set)?;
        }

        Ok(())
    }

    /// Initializes the progress bar for downloading process.
    fn initialize_progress_bar(&mut self, sets: &mut Vec<PostSet>, single_set: &mut PostSet) {
        let total_length = get_length_posts(&sets, &single_set);
        self.progress_bar
            .set_draw_target(ProgressDrawTarget::stderr());
        self.progress_bar.set_length(total_length);
        self.progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} {eta}",
                )
                .progress_chars("=>-"),
        );
    }

    /// Downloads tuple of general posts and single posts.
    pub fn download_grabbed_posts(
        &mut self,
        grabbed_sets: (Vec<PostSet>, PostSet),
    ) -> Result<(), Error> {
        let (mut sets, mut single_set) = grabbed_sets;
        self.initialize_progress_bar(&mut sets, &mut single_set);
        self.download_sets(&mut sets)?;
        self.download_set(&mut single_set)?;

        self.progress_bar.finish_and_clear();

        Ok(())
    }
}
