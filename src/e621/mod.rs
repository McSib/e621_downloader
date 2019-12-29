extern crate chrono;
extern crate failure;
extern crate pbr;
extern crate serde;

use std::fs::{create_dir_all, File};
use std::io::{stdin, Write};
use std::path::Path;

use failure::Error;
use pbr::ProgressBar;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::Login;
use crate::e621::sender::RequestSender;
use io::tag::{Group, Parsed, Tag};
use io::Config;
use reqwest::Url;
use sender::{PoolEntry, PostEntry, SetEntry};
use serde_json::Value;

pub mod blacklist;
pub mod io;
pub mod sender;

/// A collection of posts with a name.
#[derive(Debug, Clone)]
struct NamedPost {
    /// The name of the collection
    pub name: String,
    pub post_type: String,
    /// All of the post in collection
    pub posts: Vec<PostEntry>,
}

impl NamedPost {
    /// Creates a new `NamedPost` with a name.
    pub fn new(name: String, post_type: String) -> Self {
        NamedPost {
            name,
            post_type,
            posts: vec![],
        }
    }
}

impl From<&str> for NamedPost {
    fn from(name: &str) -> Self {
        NamedPost {
            name: name.to_string(),
            post_type: String::new(),
            posts: vec![],
        }
    }
}

impl Default for NamedPost {
    /// Default configuration for `NamedPost` with an empty name.
    fn default() -> Self {
        NamedPost::new(String::new(), String::new())
    }
}

impl From<&(&str, &str, &[PostEntry])> for NamedPost {
    /// Takes a tuple and creates a `NamedPost` to return.
    fn from(entry: &(&str, &str, &[PostEntry])) -> Self {
        let (name, post_type, posts) = entry;
        NamedPost {
            name: name.to_string(),
            post_type: post_type.to_string(),
            posts: posts.to_vec(),
        }
    }
}

/// A collection of `Vec<NamedPost>` and `Vec<PostEntry>`.
#[derive(Debug)]
pub struct Collection {
    /// All named posts
    named_posts: Vec<NamedPost>,
    /// All individual posts
    single_posts: NamedPost,
}

impl Default for Collection {
    fn default() -> Self {
        Collection {
            named_posts: Vec::new(),
            single_posts: NamedPost::from("Single Posts"),
        }
    }
}

/// Grabs all posts under a set of searching tags.
struct Grabber {
    /// All grabbed posts
    pub grabbed_posts: Collection,
    request_sender: RequestSender,
    /// Blacklist used to throwaway posts that contain tags the user may not want
    blacklist: Vec<String>,
}

impl Grabber {
    /// Creates new instance of `Self`.
    pub fn new(request_sender: RequestSender, blacklist: Vec<String>) -> Self {
        Grabber {
            grabbed_posts: Collection::default(),
            request_sender,
            blacklist,
        }
    }

    /// Gets posts on creation using `tags` and searching with `urls`.
    /// Also modifies the `config` when searching general tags.
    pub fn from_tags(
        groups: &[Group],
        request_sender: RequestSender,
        blacklist: Vec<&str>,
    ) -> Result<Grabber, Error> {
        let mut grabber = Grabber::new(
            request_sender,
            blacklist.iter().map(|e| e.to_string()).collect(),
        );
        grabber.grab_favorites()?;
        grabber.grab_tags(groups)?;
        Ok(grabber)
    }

    pub fn grab_favorites(&mut self) -> Result<(), Error> {
        let login = Login::load()?;
        if !login.username.is_empty() && login.download_favorites {
            let tag_str = format!("fav:{}", login.username);
            let posts = self.custom_search(tag_str.as_str())?;
            self.grabbed_posts.named_posts.push(NamedPost::from(&(
                tag_str.as_str(),
                "Favorites",
                posts.as_slice(),
            )));
            println!("\"{}\" grabbed!", tag_str);
        }

        Ok(())
    }

    /// Iterates through tags and perform searches for each, grabbing them and storing them in `self.grabbed_tags`.
    pub fn grab_tags(&mut self, groups: &[Group]) -> Result<(), Error> {
        for group in groups {
            for tag in &group.tags {
                match tag {
                    Parsed::Pool(id) => {
                        let entry: PoolEntry = self.request_sender.get_entry_from_id(id, "pool")?;
                        self.grabbed_posts.named_posts.push(NamedPost::from(&(
                            entry.name.as_str(),
                            "Pools",
                            entry.posts.as_slice(),
                        )));

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Set(id) => {
                        let entry: SetEntry = self.request_sender.get_entry_from_id(id, "set")?;
                        self.grabbed_posts
                            .named_posts
                            .push(self.set_to_named_entry(&entry)?);

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Post(id) => {
                        let entry: PostEntry =
                            self.request_sender.get_entry_from_id(id, "single")?;
                        let id = entry.id;
                        self.grabbed_posts.single_posts.posts.push(entry);

                        println!("Post with ID \"{}\" grabbed!", id);
                    }
                    Parsed::General(tag) => {
                        let tag_str = match tag {
                            Tag::General(tag_str) => tag_str.clone(),
                            Tag::Special(tag_str) => tag_str.clone(),
                            Tag::None => String::new(),
                        };
                        let posts = self.get_posts_from_tag(tag)?;
                        self.grabbed_posts.named_posts.push(NamedPost::from(&(
                            tag_str.as_str(),
                            "General Searches",
                            posts.as_slice(),
                        )));
                        println!("\"{}\" grabbed!", tag_str);
                    }
                };
            }
        }

        Ok(())
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&mut self, tag: &Tag) -> Result<Vec<PostEntry>, Error> {
        match tag {
            Tag::General(ref tag_search) => Ok(self.general_search(tag_search)?),
            Tag::Special(ref tag_search) => Ok(self.special_search(&mut tag_search.clone())?),
            Tag::None => bail!(format_err!("The tag is none!")),
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(&mut self, searching_tag: &str) -> Result<Vec<PostEntry>, Error> {
        let limit: u16 = 5;
        let mut posts: Vec<PostEntry> = Vec::with_capacity(320 * limit as usize);
        for page in 1..limit {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page)?;
            if searched_posts.is_empty() {
                break;
            }

            if !self.blacklist.is_empty() {
                let blacklist = Blacklist::new(&self.blacklist);
                blacklist.filter_posts(&mut searched_posts);
            }

            posts.append(&mut searched_posts);
        }

        Ok(posts)
    }

    /// Performs a special search that grabs from a date up to the current day.
    fn special_search(&mut self, searching_tag: &mut String) -> Result<Vec<PostEntry>, Error> {
        let mut page: u16 = 1;
        let mut posts: Vec<PostEntry> = vec![];
        loop {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page)?;
            if searched_posts.is_empty() {
                break;
            }

            if !self.blacklist.is_empty() {
                let blacklist = Blacklist::new(self.blacklist.as_slice());
                blacklist.filter_posts(&mut searched_posts);
            }

            posts.append(&mut searched_posts);
            page += 1;
        }

        Ok(posts)
    }

    /// Converts `SetEntry` to `NamedPost`.
    fn set_to_named_entry(&self, set: &SetEntry) -> Result<NamedPost, Error> {
        let posts: Vec<PostEntry> = self.custom_search(&format!("set:{}", set.short_name))?;
        Ok(NamedPost::from(&(
            set.name.as_str(),
            "Sets",
            posts.as_slice(),
        )))
    }

    fn custom_search(&self, tag: &str) -> Result<Vec<PostEntry>, Error> {
        let mut posts = vec![];
        let mut page = 1;
        loop {
            let mut set_posts: Vec<PostEntry> = self.request_sender.bulk_search(tag, page)?;
            if set_posts.is_empty() {
                break;
            }

            posts.append(&mut set_posts);
            page += 1;
        }

        Ok(posts)
    }
}

pub struct WebConnector<'a> {
    request_sender: RequestSender,
    /// The config which is modified when grabbing posts
    config: &'a mut Config,
    /// Login information for grabbing the Blacklist
    login: &'a Login,
    /// Blacklist grabbed from logged in user
    blacklist: String,
}

impl<'a> WebConnector<'a> {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(config: &'a mut Config, login: &'a Login, request_sender: RequestSender) -> Self {
        WebConnector {
            request_sender,
            config,
            login,
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
        if !self.login.is_empty() {
            let json: Value = self.request_sender.grab_blacklist(self.login)?;
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
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<Collection, Error> {
        Ok(Grabber::from_tags(
            groups,
            self.request_sender.clone(),
            self.blacklist.lines().collect::<Vec<&str>>(),
        )?
        .grabbed_posts)
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
        name: &mut String,
        post_type: &str,
        posts: &[PostEntry],
    ) -> Result<(), Error> {
        let mut progress_bar = ProgressBar::new(posts.len() as u64);
        for post in posts {
            self.remove_invalid_chars(name);
            progress_bar.message(format!("Downloading: {} ", name).as_str());

            let file_name = Url::parse(post.file_url.as_str())?
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .to_string();
            let file_dir = if post_type.is_empty() {
                format!("{}{}/", self.config.download_directory, name)
            } else {
                format!("{}{}/{}/", self.config.download_directory, post_type, name)
            };

            let file_path_string = format!("{}{}", file_dir, file_name);
            let file_path = Path::new(file_path_string.as_str());
            if file_path.exists() {
                progress_bar.message("Duplicate found: skipping... ");
                continue;
            }

            if !Path::new(file_dir.as_str()).exists() {
                create_dir_all(file_dir)?;
            }

            //            let bytes = self.download_post(&post.file_url, post.file_size.unwrap_or(0))?;
            let bytes = self
                .request_sender
                .download_image(&post.file_url, post.file_size.unwrap_or(0))?;
            self.save_image(file_path, &bytes)?;

            progress_bar.inc();
        }

        progress_bar.finish_println("");
        Ok(())
    }

    /// Downloads all posts from collection.
    pub fn download_posts_from_collection(
        &mut self,
        collection: &mut Collection,
    ) -> Result<(), Error> {
        self.download_singles(&mut collection.single_posts)?;
        self.download_named(&mut collection.named_posts)?;

        Ok(())
    }

    fn download_singles(&mut self, single_posts: &mut NamedPost) -> Result<(), Error> {
        self.download_posts(
            &mut single_posts.name,
            &single_posts.post_type,
            &single_posts.posts,
        )?;

        Ok(())
    }

    fn download_named(&mut self, named_posts: &mut Vec<NamedPost>) -> Result<(), Error> {
        for named_post in named_posts {
            self.download_posts(
                &mut named_post.name,
                &named_post.post_type,
                &named_post.posts,
            )?;
        }

        Ok(())
    }
}
