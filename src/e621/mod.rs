extern crate indicatif;

use std::cell::RefCell;
use std::fs::{create_dir_all, write};
use std::io::stdin;
use std::path::PathBuf;
use std::rc::Rc;

use indicatif::{ProgressDrawTarget, ProgressStyle};
use indicatif::ProgressBar;

use blacklist::Blacklist;
use grabber::Grabber;
use io::Config;
use io::tag::Group;
use sender::{RequestSender, UserEntry};

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
		println!("Should enter safe mode? [y/N]");

		loop {
			let mut input = String::new();
			stdin().read_line(&mut input).expect("Unable to read line!");
			match input.as_str().trim() {
				"y" | "Y" => {
					self.request_sender.update_to_safe();
					break;
				}
				"n" | "N" | "" => break,
				_ => {}
			}
		}

		// if confirmation_result.unwrap_or_default() {
		//     self.request_sender.update_to_safe();
		// }
	}

	/// Processes the blacklist and tokenizes for use when grabbing posts.
	pub fn process_blacklist(&mut self, username: &str) {
		let user: UserEntry = self
			.request_sender
			.get_entry_from_appended_id(username, "user");
		if let Some(blacklist_tags) = user.blacklisted_tags {
			if !blacklist_tags.is_empty() {
				self.blacklist.borrow_mut().parse_blacklist(blacklist_tags);
				self.blacklist.borrow_mut().cache_users();
				self.grabber.set_blacklist(self.blacklist.clone());
			}
		}
	}

	/// Creates `Grabber` and grabs all posts before returning a tuple containing all general posts and single posts (posts grabbed by its ID).
	pub fn grab_posts(&mut self, groups: &[Group]) {
		self.grabber.grab_favorites();
		self.grabber.grab_posts_by_tags(groups);
	}

	/// Saves image to download directory.
	fn save_image(&self, file_path: &str, bytes: &[u8]) {
		write(file_path, bytes).expect("Failed to save image!");
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
			for post in &collection.posts {
				self.progress_bar
					.set_message(&format!("Downloading: {} ", collection.name));
				let file_path: PathBuf = [
					&self.download_directory,
					&collection.category,
					&collection.name,
					&post.name,
				]
					.iter()
					.map(|e| self.remove_invalid_chars(e))
					.collect();
				create_dir_all(file_path.parent().unwrap())
					.expect("Could not create directories for images!");
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
				.progress_chars("=>-"),
		);
		self.progress_bar
			.set_draw_target(ProgressDrawTarget::stderr());
		self.progress_bar.reset();
		self.progress_bar.enable_steady_tick(100);
	}

	/// Downloads tuple of general posts and single posts.
	pub fn download_posts(&mut self) {
		// Initializes the progress bar for downloading.
		let length = self.get_total_file_size();
		self.initialize_progress_bar(length);
		self.download_collection();

		self.progress_bar.finish_and_clear();
	}

	/// Gets the total size (in KB) of every post image to be downloaded.
	fn get_total_file_size(&mut self) -> u64 {
		self.grabber
			.posts
			.iter()
			.map(|e| e.posts.iter().map(|f| f.file_size).sum::<i64>())
			.sum::<i64>() as u64
	}
}
