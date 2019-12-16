extern crate chrono;
extern crate failure;
extern crate pbr;
extern crate reqwest;
extern crate serde;

use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{stdin, Read, Write};
use std::path::Path;

use failure::Error;
use pbr::ProgressBar;
use reqwest::{header::USER_AGENT, Client, RequestBuilder, Url};
use serde::Serialize;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::{emergency_exit, Login};
use data_sets::{PoolEntry, PostEntry, SetEntry};
use io::tag::{Group, Parsed, Tag};
use io::Config;
use serde_json::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub mod blacklist;
mod data_sets;
pub mod io;

/// Default user agent value.
static USER_AGENT_VALUE: &str = "e621_downloader/1.3.3 (by McSib on e621)";

/// A collection of posts with a name.
#[derive(Debug, Clone)]
struct NamedPost {
    /// The name of the collection
    pub name: String,
    /// All of the post in collection
    pub posts: Vec<PostEntry>,
}

impl NamedPost {
    /// Creates a new `NamedPost` with a name.
    pub fn new(name: String) -> Self {
        NamedPost {
            name,
            posts: vec![],
        }
    }
}

impl From<&str> for NamedPost {
    fn from(name: &str) -> Self {
        NamedPost {
            name: name.to_string(),
            posts: vec![],
        }
    }
}

impl Default for NamedPost {
    /// Default configuration for `NamedPost` with an empty name.
    fn default() -> Self {
        NamedPost::new(String::new())
    }
}

impl From<&(&str, &[PostEntry])> for NamedPost {
    /// Takes a tuple and creates a `NamedPost` to return.
    fn from(entry: &(&str, &[PostEntry])) -> Self {
        NamedPost {
            name: entry.0.to_string(),
            posts: entry.1.to_vec(),
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
    /// Urls given as reference by `EsixWebConnector`
    urls: Rc<RefCell<HashMap<String, String>>>,
    /// Blacklist used to throwaway posts that contain tags the user may not want
    blacklist: Vec<String>,
}

impl Grabber {
    /// Creates new instance of `Self`.
    pub fn new(urls: Rc<RefCell<HashMap<String, String>>>, blacklist: Vec<String>) -> Self {
        Grabber {
            grabbed_posts: Collection::default(),
            urls,
            blacklist,
        }
    }

    /// Gets posts on creation using `tags` and searching with `urls`.
    /// Also modifies the `config` when searching general tags.
    pub fn from_tags(
        groups: &[Group],
        urls: Rc<RefCell<HashMap<String, String>>>,
        blacklist: Vec<&str>,
    ) -> Result<Grabber, Error> {
        let mut grabber = Grabber::new(urls, blacklist.iter().map(|e| e.to_string()).collect());
        grabber.grab_favorites()?;
        grabber.grab_tags(groups)?;
        Ok(grabber)
    }

    pub fn grab_favorites(&mut self) -> Result<(), Error> {
        let login = Login::load()?;
        if !login.username.is_empty() && login.download_favorites {
            let tag_str = format!("fav:{}", login.username);
            let tag_client = Client::new();
            let posts = self.custom_search(&tag_client, tag_str.as_str())?;
            self.grabbed_posts
                .named_posts
                .push(NamedPost::from(&(tag_str.as_str(), posts.as_slice())));
            println!("\"{}\" grabbed!", tag_str);
        }

        Ok(())
    }

    /// Iterates through tags and perform searches for each, grabbing them and storing them in `self.grabbed_tags`.
    pub fn grab_tags(&mut self, groups: &[Group]) -> Result<(), Error> {
        let tag_client = Client::new();
        for group in groups {
            for tag in &group.tags {
                match tag {
                    Parsed::Pool(id) => {
                        let entry: PoolEntry = self
                            .get_request_builder(&tag_client, "pool", &[("id", id)])
                            .send()?
                            .json()?;
                        self.grabbed_posts.named_posts.push(NamedPost::from(&(
                            entry.name.as_str(),
                            entry.posts.as_slice(),
                        )));

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Set(id) => {
                        let entry: SetEntry = self
                            .get_request_builder(&tag_client, "set", &[("id", id)])
                            .send()?
                            .json()?;
                        self.grabbed_posts
                            .named_posts
                            .push(self.set_to_named_entry(&entry)?);

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Post(id) => {
                        let entry: PostEntry = self
                            .get_request_builder(&tag_client, "single", &[("id", id)])
                            .send()?
                            .json()?;
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
                        let posts = self.get_posts_from_tag(&tag_client, tag)?;
                        self.grabbed_posts
                            .named_posts
                            .push(NamedPost::from(&(tag_str.as_str(), posts.as_slice())));
                        println!("\"{}\" grabbed!", tag_str);
                    }
                };
            }
        }

        Ok(())
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&mut self, client: &Client, tag: &Tag) -> Result<Vec<PostEntry>, Error> {
        match tag {
            Tag::General(ref tag_search) => Ok(self.general_search(client, tag_search)?),
            Tag::Special(ref tag_search) => {
                Ok(self.special_search(client, &mut tag_search.clone())?)
            }
            Tag::None => bail!(format_err!("The tag is none!")),
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(
        &mut self,
        client: &Client,
        searching_tag: &str,
    ) -> Result<Vec<PostEntry>, Error> {
        let limit: u8 = 5;
        let mut posts: Vec<PostEntry> = Vec::with_capacity(320 * limit as usize);
        for page in 1..limit {
            let mut searched_posts: Vec<PostEntry> = self
                .get_request_builder(
                    &client,
                    "post",
                    &[
                        ("tags", searching_tag),
                        ("page", &format!("{}", page)),
                        ("limit", &format!("{}", 320)),
                    ],
                )
                .send()?
                .json::<Vec<PostEntry>>()?;
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
    fn special_search(
        &mut self,
        client: &Client,
        searching_tag: &mut String,
    ) -> Result<Vec<PostEntry>, Error> {
        let mut page: u16 = 1;
        let mut posts: Vec<PostEntry> = vec![];
        loop {
            let mut searched_posts: Vec<PostEntry> = self
                .get_request_builder(
                    &client,
                    "post",
                    &[
                        ("tags", &*searching_tag),
                        ("page", &format!("{}", page)),
                        ("limit", &format!("{}", 320)),
                    ],
                )
                .send()?
                .json()?;
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

    /// Creates a request builder for tag searches.
    fn get_request_builder<T: Serialize>(
        &self,
        client: &Client,
        entry: &str,
        query: &T,
    ) -> RequestBuilder {
        client
            .get(self.urls.borrow()[entry].as_str())
            .header(USER_AGENT, USER_AGENT_VALUE)
            .query(query)
    }

    /// Converts `SetEntry` to `NamedPost`.
    fn set_to_named_entry(&self, set: &SetEntry) -> Result<NamedPost, Error> {
        let client = Client::new();
        let posts: Vec<PostEntry> =
            self.custom_search(&client, format!("set:{}", set.name).as_str())?;
        Ok(NamedPost::from(&(set.name.as_str(), posts.as_slice())))
    }

    fn custom_search(&self, client: &Client, tag: &str) -> Result<Vec<PostEntry>, Error> {
        let mut posts = vec![];
        let mut page = 1;
        loop {
            let mut set_posts: Vec<PostEntry> = self
                .get_request_builder(
                    &client,
                    "post",
                    &[
                        ("tags", tag),
                        ("page", format!("{}", page).as_str()),
                        ("limit", "320"),
                    ],
                )
                .send()?
                .json()?;
            if set_posts.is_empty() {
                break;
            }

            posts.append(&mut set_posts);
            page += 1;
        }

        Ok(posts)
    }
}

pub struct EsixWebConnector<'a> {
    /// All urls that can be used.
    /// These options are `"post"`, `"pool"`, `"set"`, and `"single"`
    urls: Rc<RefCell<HashMap<String, String>>>,
    /// The config which is modified when grabbing posts
    config: &'a mut Config,
    /// Client used for downloading posts
    client: Client,
    /// Login information for grabbing the Blacklist
    login: &'a Login,
    /// Blacklist grabbed from logged in user
    blacklist: String,
}

impl<'a> EsixWebConnector<'a> {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(config: &'a mut Config, login: &'a Login) -> Self {
        let mut urls: HashMap<String, String> = HashMap::new();
        EsixWebConnector::insert_urls(&mut urls);

        EsixWebConnector {
            urls: Rc::new(RefCell::new(urls)),
            config,
            client: Client::new(),
            login,
            blacklist: String::new(),
        }
    }

    fn insert_urls(urls: &mut HashMap<String, String>) {
        urls.insert(
            String::from("post"),
            String::from("https://e621.net/post/index.json"),
        );
        urls.insert(
            String::from("pool"),
            String::from("https://e621.net/pool/show.json"),
        );
        urls.insert(
            String::from("set"),
            String::from("https://e621.net/set/show.json"),
        );
        urls.insert(
            String::from("single"),
            String::from("https://e621.net/post/show.json"),
        );
    }

    /// Gets input and checks if the user wants to enter safe mode.
    /// If they do, this changes `self.urls` all to e926 and not e621.
    pub fn should_enter_safe_mode(&mut self) {
        if self.get_input("Should enter safe mode") {
            self.update_urls_to_safe();
        }
    }

    pub fn grab_blacklist(&mut self) -> Result<(), Error> {
        if !self.login.is_empty() {
            let url = "https://e621.net/user/blacklist.json";
            let json: Value = self
                .client
                .get(url)
                .query(&[
                    ("login", self.login.username.as_str()),
                    ("password_hash", self.login.password_hash.as_str()),
                ])
                .send()?
                .json()?;
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

    /// Updates all urls from e621 to e926.
    fn update_urls_to_safe(&mut self) {
        self.urls
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, val)| *val = val.replace("e621", "e926"));
    }

    /// Grabs all posts using `&[Group]` then converts grabbed posts and appends it to `self.collection`.
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<Collection, Error> {
        Ok(Grabber::from_tags(
            groups,
            self.urls.clone(),
            self.blacklist.lines().collect::<Vec<&str>>(),
        )?
        .grabbed_posts)
    }

    /// Saves image to download directory.
    fn save_image(
        &mut self,
        dir_name: &mut String,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<(), Error> {
        self.remove_invalid_chars(dir_name);
        let file_dir = if self.config.create_directories {
            format!("{}{}", self.config.download_directory, dir_name)
        } else {
            self.config.download_directory.clone()
        };
        let dir = Path::new(file_dir.as_str());
        if !dir.exists() {
            create_dir_all(dir)?;
        }

        let mut image_file: File = File::create(dir.join(file_name))?;
        image_file.write_all(bytes)?;

        Ok(())
    }

    /// Removes invalid characters from directory name.
    fn remove_invalid_chars(&self, dir_name: &mut String) {
        for character in &["\\", "/", "?", ":", "*", "<", ">", "\"", "|"] {
            *dir_name = dir_name.replace(character, "_");
        }
    }

    /// Sends request to download image.
    fn download_post(&self, url: &str, file_size: i64) -> Result<(String, Vec<u8>), Error> {
        let image_result = self
            .client
            .get(url)
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send();
        let mut image_response = match image_result {
            Ok(response) => response,
            Err(ref error) => {
                self.close_on_server_error(error);
                unreachable!()
            }
        };

        let mut image_bytes: Vec<u8> = Vec::with_capacity(file_size as usize);
        image_response.read_to_end(&mut image_bytes)?;

        Ok((
            image_response
                .url()
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .to_string(),
            image_bytes,
        ))
    }

    fn close_on_server_error(&self, error: &reqwest::Error) {
        println!(
            "The server returned a {} error code!",
            error.status().unwrap()
        );
        if error.is_server_error() {
            emergency_exit(
                "If this code is 503, please contact the developer (McSib) and report this to him.",
            );
        } else {
            println!("If error code is 4xx, this is a client side error.");
            emergency_exit(
                "Please contact the developer (McSib) about this problem if it is 403, 404, 421",
            );
        }
    }

    /// Processes vec and downloads all posts from it.
    fn download_posts(&mut self, mut name: String, posts: &[PostEntry]) -> Result<(), Error> {
        let mut progress_bar = ProgressBar::new(posts.len() as u64);
        for post in posts {
            progress_bar.message(format!("Downloading: {} ", name).as_str());
            let file_name = Url::parse(post.file_url.as_str())?
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .to_string();
            let file_dir = format!("{}{}/{}", self.config.download_directory, name, file_name);
            if Path::new(file_dir.as_str()).exists() {
                progress_bar.message("Duplicate found: skipping... ");
                continue;
            }
            let (file_name, bytes) =
                self.download_post(&post.file_url, post.file_size.unwrap_or(0))?;
            self.save_image(&mut name, &file_name, &bytes)?;
            progress_bar.inc();
        }

        progress_bar.finish_println("");
        Ok(())
    }

    /// Downloads all posts from collection.
    pub fn download_posts_from_collection(&mut self, collection: &Collection) -> Result<(), Error> {
        self.download_singles(collection)?;
        self.download_named(collection)?;

        Ok(())
    }

    fn download_singles(&mut self, collection: &Collection) -> Result<(), Error> {
        let single_posts = &collection.single_posts;
        self.download_posts(single_posts.name.clone(), &single_posts.posts)?;

        Ok(())
    }

    fn download_named(&mut self, collection: &Collection) -> Result<(), Error> {
        for named_post in &collection.named_posts {
            self.download_posts(named_post.name.clone(), &named_post.posts)?;
        }

        Ok(())
    }
}
