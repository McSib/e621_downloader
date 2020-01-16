extern crate dialoguer;
extern crate failure;
extern crate indicatif;

use std::fs::{create_dir_all, write};
use std::path::PathBuf;

use dialoguer::Confirmation;
use failure::Error;
use indicatif::ProgressBar;

use io::tag::Group;
use io::Config;

use crate::e621::grabber::{Grabber, PostSet};
use crate::e621::sender::RequestSender;

use self::indicatif::{ProgressDrawTarget, ProgressStyle};

pub mod blacklist;
pub mod grabber;
pub mod io;
pub mod sender;

/// Get the total file size from all sets and returns it.
fn get_file_size_from_posts(sets: &[PostSet], single_set: &PostSet) -> u64 {
    let sets_total_size: i64 = sets
        .iter()
        .map(|e| e.posts.iter().map(|f| f.file_size).sum::<i64>())
        .sum();
    let single_total_size: i64 = single_set.posts.iter().map(|e| e.file_size).sum();
    (sets_total_size + single_total_size) as u64
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
        let config = Config::get_config().unwrap_or_default();
        WebConnector {
            request_sender: request_sender.clone(),
            download_directory: config.download_directory,
            progress_bar: ProgressBar::hidden(),
        }
    }

    /// Gets input and checks if the user wants to enter safe mode.
    /// If they do, the `RequestSender` will update the request urls for future sent requests.
    pub fn should_enter_safe_mode(&mut self) {
        let confirmation_result = Confirmation::new()
            .with_text("Should enter safe mode?")
            .default(false)
            .interact();
        if confirmation_result.unwrap_or_default() {
            self.request_sender.update_to_safe();
        }
    }

    /// Creates `Grabber` and grabs all posts before returning a tuple containing all general posts and single posts (posts grabbed by its ID).
    pub fn grab_posts(&mut self, groups: &[Group]) -> (Vec<PostSet>, PostSet) {
        let grabber = Grabber::from_tags(groups, self.request_sender.clone());
        (grabber.grabbed_posts, grabber.grabbed_single_posts)
    }

    /// Saves image to download directory.
    fn save_image(&mut self, file_path: &str, bytes: &[u8]) {
        write(file_path, bytes).expect("Failed to save image!");
    }

    /// Removes invalid characters from directory name.
    fn remove_invalid_chars(&self, dir_name: &mut String) {
        *dir_name = dir_name
            .chars()
            .map(|e| match e {
                '?' | ':' | '*' | '<' | '>' | '\"' | '|' => '_',
                _ => e,
            })
            .collect();
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
            create_dir_all(file_path.parent().unwrap())?;
            if file_path.exists() {
                self.progress_bar
                    .set_message("Duplicate found: skipping... ");
                self.progress_bar.inc(post.file_size as u64);
                continue;
            }

            let bytes = self
                .request_sender
                .download_image(&post.file_url, post.file_size);
            self.save_image(file_path.to_str().unwrap(), &bytes);
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
        let total_length = get_file_size_from_posts(&sets, &single_set);
        self.progress_bar.set_length(total_length);
        self.progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} {eta}",
                )
                .progress_chars("=>-"),
        );
        self.progress_bar
            .set_draw_target(ProgressDrawTarget::stderr());
        self.progress_bar.reset();
        self.progress_bar.enable_steady_tick(100);
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
