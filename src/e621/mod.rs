extern crate failure;
extern crate indicatif;

use std::fs::{create_dir_all, File};
use std::io::{stdin, Write};
use std::path::Path;

use failure::Error;
use indicatif::ProgressBar;

use io::tag::Group;
use io::Config;

use self::indicatif::ProgressStyle;
use crate::e621::grabber::{Grabber, PostSet};
use crate::e621::sender::RequestSender;

pub mod blacklist;
pub mod grabber;
pub mod io;
pub mod sender;

fn get_total_posts(sets: &[PostSet]) -> u64 {
    let mut total_posts = 0;
    for set in sets {
        for post in &set.posts {
            total_posts += post.file_size as u64;
        }
    }

    total_posts
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
        if self.get_input("Should enter safe mode") {
            self.request_sender.update_to_safe();
        }
    }

    /// Gets a simple yes/no for whether or not to do something.
    fn get_input(&self, msg: &str) -> bool {
        println!("{} (Y/N)?", msg);
        loop {
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            match input.to_lowercase().trim() {
                "y" | "yes" => return true,
                "n" | "no" => return false,
                _ => {
                    println!("Incorrect input!");
                    println!("Try again!");
                }
            }
        }
    }

    /// Creates `Grabber` and grabs all posts before returning a tuple containing all general posts and single posts (posts grabbed by its ID).
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<(Vec<PostSet>, PostSet), Error> {
        let grabber = Grabber::from_tags(groups, self.request_sender.clone())?;
        Ok((grabber.grabbed_posts, grabber.grabbed_single_posts))
    }

    /// Saves image to download directory.
    fn save_image(&mut self, file_path: &Path, bytes: &[u8]) -> Result<(), Error> {
        let mut image_file: File = File::create(file_path)?;
        image_file.write_all(bytes)?;

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
        // TODO: Do a better job at making this function understandable and idiomatic.
        self.remove_invalid_chars(&mut set.set_name);
        set.posts.reverse();
        for post in &set.posts {
            self.progress_bar
                .set_message(format!("Downloading: {} ", set.set_name).as_str());
            let file_dir = if set.category.is_empty() {
                format!("{}{}/", self.download_directory, set.set_name)
            } else {
                format!(
                    "{}{}/{}/",
                    self.download_directory, set.category, set.set_name
                )
            };

            let file_path_string = format!("{}{}", file_dir, post.file_name);
            let file_path = Path::new(file_path_string.as_str());
            if file_path.exists() {
                self.progress_bar
                    .set_message("Duplicate found: skipping... ");
                continue;
            }

            if !Path::new(file_dir.as_str()).exists() {
                create_dir_all(file_dir)?;
            }

            let bytes = self
                .request_sender
                .download_image(&post.file_url, post.file_size)?;
            self.save_image(file_path, &bytes)?;

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

    /// Downloads tuple of general posts and single posts.
    pub fn download_grabbed_posts(
        &mut self,
        grabbed_sets: (Vec<PostSet>, PostSet),
    ) -> Result<(), Error> {
        let (mut sets, mut single_set) = grabbed_sets;
        let mut total_length = get_total_posts(&sets);
        single_set
            .posts
            .iter()
            .for_each(|e| total_length += e.file_size as u64);

        self.progress_bar = ProgressBar::new(total_length);
        self.progress_bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{msg} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} {bytes_per_sec} {eta}",
                )
                .progress_chars("=>-"),
        );

        self.download_sets(&mut sets)?;
        self.download_set(&mut single_set)?;

        self.progress_bar.finish_and_clear();

        Ok(())
    }
}
