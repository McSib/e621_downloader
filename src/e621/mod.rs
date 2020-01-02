extern crate chrono;
extern crate failure;
extern crate pbr;
extern crate serde;

use std::fs::{create_dir_all, File};
use std::io::{stdin, Write};
use std::path::Path;

use failure::Error;
use pbr::ProgressBar;

use crate::e621::grabber::{GrabbedPost, Grabber, PostSet};
use crate::e621::io::Login;
use crate::e621::sender::RequestSender;
use io::tag::Group;
use io::Config;
use serde_json::Value;

pub mod blacklist;
pub mod grabber;
pub mod io;
pub mod sender;

pub struct WebConnector {
    request_sender: RequestSender,
    /// The config which is modified when grabbing posts
    download_directory: String,
    /// Blacklist grabbed from logged in user
    blacklist: String,
}

impl WebConnector {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(request_sender: RequestSender) -> Self {
        WebConnector {
            request_sender,
            download_directory: Config::get_config().unwrap_or_default().download_directory,
            blacklist: String::new(),
        }
    }

    /// Gets input and checks if the user wants to enter safe mode.
    /// If they do, this changes `self.urls` all to e926 and not e621.
    pub fn should_enter_safe_mode(&mut self) {
        if self.get_input("Should enter safe mode") {
            self.request_sender.update_to_safe();
        }
    }

    pub fn grab_blacklist(&mut self) -> Result<(), Error> {
        let login = Login::load()?;
        if !login.is_empty() {
            let json: Value = self.request_sender.get_blacklist(&login)?;
            self.blacklist = json["blacklist"]
                .to_string()
                .trim_matches('\"')
                .replace("\\n", "\n");
        }

        Ok(())
    }

    /// Gets simply a yes/no for whether or not to do something.
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

    /// Grabs all posts using `&[Group]` then converts grabbed posts and appends it to `self.collection`.
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<(Vec<PostSet>, PostSet), Error> {
        let grabber = Grabber::from_tags(
            groups,
            self.request_sender.clone(),
            self.blacklist.lines().collect::<Vec<&str>>(),
        )?;
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

    /// Processes vec and downloads all posts from it.
    fn download_posts(
        &mut self,
        set_name: &mut String,
        category: &str,
        posts: &mut [GrabbedPost],
    ) -> Result<(), Error> {
        let mut progress_bar = ProgressBar::new(posts.len() as u64);
        posts.reverse();
        for post in posts {
            self.remove_invalid_chars(set_name);
            progress_bar.message(format!("Downloading: {} ", set_name).as_str());
            let file_dir = if category.is_empty() {
                format!("{}{}/", self.download_directory, set_name)
            } else {
                format!("{}{}/{}/", self.download_directory, category, set_name)
            };

            let file_path_string = format!("{}{}", file_dir, post.file_name);
            let file_path = Path::new(file_path_string.as_str());
            if file_path.exists() {
                progress_bar.message("Duplicate found: skipping... ");
                continue;
            }

            if !Path::new(file_dir.as_str()).exists() {
                create_dir_all(file_dir)?;
            }

            let bytes = self
                .request_sender
                .download_image(&post.file_url, post.file_size)?;
            self.save_image(file_path, &bytes)?;

            progress_bar.inc();
        }

        progress_bar.finish_println("");
        Ok(())
    }

    pub fn download_grabbed_posts(
        &mut self,
        grabbed_posts: (Vec<PostSet>, PostSet),
    ) -> Result<(), Error> {
        let (mut posts, mut single_posts) = grabbed_posts;
        for post in posts.iter_mut() {
            self.download_posts(
                &mut post.set_name.clone(),
                &post.category.to_string(),
                &mut post.posts,
            )?;
        }

        self.download_posts(
            &mut single_posts.set_name.clone(),
            &single_posts.category.to_string(),
            &mut single_posts.posts,
        )?;
        Ok(())
    }
}
