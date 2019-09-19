extern crate chrono;
extern crate failure;
extern crate pbr;
extern crate reqwest;
extern crate serde;

use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{stdin, Read, Write};
use std::path::Path;

use chrono::Local;
use failure::Error;
use pbr::ProgressBar;
use reqwest::{header::USER_AGENT, Client, RequestBuilder, Url};
use serde::Serialize;

use crate::e621::io::emergency_exit;
use data_sets::{PoolEntry, PostEntry, SetEntry};
use io::tag::{Group, Parsed, Tag};
use io::Config;

mod data_sets;
pub mod io;

/// Default user agent value.
static USER_AGENT_VALUE: &'static str = "e621_downloader/1.1.3 (by McSib on e621)";

/// Default date for new tags.
static DEFAULT_DATE: &'static str = "2006-01-01";

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

/// Posts grab from search.
struct GrabbedPosts {
    /// All posts from pools
    pools: Vec<NamedPost>,
    /// All posts from sets
    sets: Vec<NamedPost>,
    /// All individual posts
    singles: NamedPost,
    /// All posts under a searching tag
    posts: Vec<NamedPost>,
}

impl Default for GrabbedPosts {
    /// Default configuration for `GrabbedPosts`
    fn default() -> Self {
        GrabbedPosts {
            pools: vec![],
            sets: vec![],
            singles: NamedPost::new(String::from("single_posts")),
            posts: vec![],
        }
    }
}

/// A collection of `Vec<NamedPost>` and `Vec<PostEntry>`.
#[derive(Default, Debug)]
pub struct Collection {
    /// All named posts
    named_posts: Vec<NamedPost>,
    /// All individual posts
    single_posts: NamedPost,
}

impl From<GrabbedPosts> for Collection {
    /// Converts [`GrabbedPosts`] to `Collection`.
    fn from(mut grabbed: GrabbedPosts) -> Self {
        let mut collection = Collection::default();
        collection.named_posts.append(&mut grabbed.posts);
        collection.named_posts.append(&mut grabbed.pools);
        collection.named_posts.append(&mut grabbed.sets);
        collection.single_posts = grabbed.singles;

        collection
    }
}

/// Grabs all posts under a set of searching tags.
struct Grabber<'a> {
    /// All grabbed posts
    pub grabbed_posts: GrabbedPosts,
    /// Urls given as reference by `EsixWebConnector`
    urls: &'a HashMap<String, String>,
    /// Config given as reference by `EsixWebConnector`
    config: &'a mut Config,
}

impl<'a> Grabber<'a> {
    /// Creates new instance of `Self`.
    pub fn new(urls: &'a HashMap<String, String>, config: &'a mut Config) -> Self {
        Grabber {
            grabbed_posts: GrabbedPosts::default(),
            urls,
            config,
        }
    }

    /// Gets posts on creation using `tags` and searching with `urls`.
    /// Also modifies the `config` when searching general tags.
    pub fn from_tags(
        groups: &[Group],
        urls: &'a HashMap<String, String>,
        config: &'a mut Config,
    ) -> Result<Grabber<'a>, Error> {
        let mut grabber = Grabber::new(urls, config);
        grabber.grab_tags(groups)?;
        Ok(grabber)
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
                        self.grabbed_posts.pools.push(NamedPost::from(&(
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
                        self.grabbed_posts.sets.push(self.set_to_named(&entry)?);

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Post(id) => {
                        let entry: PostEntry = self
                            .get_request_builder(&tag_client, "single", &[("id", id)])
                            .send()?
                            .json()?;
                        let id = entry.id;
                        self.grabbed_posts.singles.posts.push(entry);

                        println!("\"{}\" post grabbed!", id);
                    }
                    Parsed::General(tag) => {
                        let tag_str = match tag {
                            Tag::General(tag_str) => tag_str.clone(),
                            Tag::Special(tag_str) => tag_str.clone(),
                            Tag::None => String::new(),
                        };
                        let posts = self.get_posts_from_tag(&tag_client, tag)?;
                        self.grabbed_posts
                            .posts
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
            Tag::General(tag_search) => Ok(self.general_search(client, tag_search)?),
            Tag::Special(tag_search) => Ok(self.special_search(client, &mut tag_search.clone())?),
            Tag::None => bail!(format_err!("The tag is none!")),
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(
        &mut self,
        client: &Client,
        searching_tag: &String,
    ) -> Result<Vec<PostEntry>, Error> {
        let limit: u8 = 5;
        let mut posts: Vec<PostEntry> = vec![];
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
        self.update_tag_date(searching_tag.as_str());
        self.add_date_to_tag(searching_tag);

        let mut page: u16 = 1;
        let mut posts = vec![];
        loop {
            let mut data_set: Vec<PostEntry> = self
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
            if data_set.is_empty() {
                break;
            }

            posts.append(&mut data_set);
            page += 1;
        }

        Ok(posts)
    }

    /// Adds date to tag.
    fn add_date_to_tag(&self, tag: &mut String) {
        *tag = format!("{} date:>={}", tag, self.config.last_run[tag.as_str()]);
    }

    /// Updates the tag date.
    fn update_tag_date(&mut self, entry: &str) {
        self.config
            .last_run
            .entry(entry.to_string())
            .and_modify(|val| *val = Local::today().format("%Y-%m-%d").to_string())
            .or_insert_with(|| DEFAULT_DATE.to_string());
    }

    /// Creates a request builder for tag searches.
    fn get_request_builder<T: Serialize>(
        &self,
        client: &Client,
        entry: &str,
        query: &T,
    ) -> RequestBuilder {
        client
            .get(self.urls[entry].as_str())
            .header(USER_AGENT, USER_AGENT_VALUE)
            .query(query)
    }

    /// Converts `SetEntry` to `NamedPost`.
    fn set_to_named(&self, set: &SetEntry) -> Result<NamedPost, Error> {
        let client = Client::new();
        let mut posts: Vec<PostEntry> = Vec::with_capacity(set.posts.len());
        let mut page = 1;
        loop {
            let mut set_posts: Vec<PostEntry> = self
                .get_request_builder(
                    &client,
                    "post",
                    &[
                        ("tags", format!("set:{}", set.short_name).as_str()),
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

        Ok(NamedPost::from(&(set.name.as_str(), posts.as_slice())))
    }
}

pub struct EsixWebConnector<'a> {
    /// All urls that can be used.
    /// These options are `"post"`, `"pool"`, `"set"`, and `"single"`
    urls: HashMap<String, String>,
    /// The config which is modified when grabbing posts
    config: &'a mut Config,
    /// Client used for downloading posts
    client: Client,
}

impl<'a> EsixWebConnector<'a> {
    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(config: &'a mut Config) -> Self {
        let mut urls: HashMap<String, String> = HashMap::new();
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

        EsixWebConnector {
            urls,
            config,
            client: Client::new(),
        }
    }

    /// Gets input and checks if the user wants to enter safe mode.
    /// If they do, this changes `self.urls` all to e926 and not e621.
    pub fn should_enter_safe_mode(&mut self) {
        if self.get_input("Should enter safe mode") {
            self.update_urls_to_safe();
        }
    }

    /// Gets a simply yes/no for whether or not to do something.
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
        for (_, val) in self.urls.iter_mut() {
            let safe = val.replace("e621", "e926");
            *val = safe;
        }
    }

    /// Grabs all posts using `&[Group]` then converts grabbed posts and appends it to `self.collection`.
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<Collection, Error> {
        let post_grabber = Grabber::from_tags(groups, &self.urls, self.config)?;
        let collection = Collection::from(post_grabber.grabbed_posts);

        Ok(collection)
    }

    /// Saves image to download directory.
    fn save_image(
        &mut self,
        dir_name: &mut String,
        file_name: &str,
        bytes: &Vec<u8>,
    ) -> Result<(), Error> {
        // Remove invalid characters from directory name.
        for character in &["\\", "/", "?", ":", "*", "<", ">", "\"", "|"] {
            *dir_name = dir_name.replace(character, "_");
        }

        let file_dir = if self.config.create_directories {
            format!("{}{}", self.config.download_directory, dir_name)
        } else {
            format!("{}", self.config.download_directory)
        };
        let dir = Path::new(file_dir.as_str());
        if !dir.exists() {
            create_dir_all(dir)?;
        }

        let mut image_file: File = File::create(dir.join(file_name))?;
        image_file.write_all(bytes.as_slice())?;

        Ok(())
    }

    /// Sends request to download image.
    fn download_post(&self, url: &String) -> Result<(String, Vec<u8>), Error> {
        let image_result = self
            .client
            .get(url)
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send();
        let mut image_response = match image_result {
            Ok(response) => response,
            Err(error) => {
                println!(
                    "The server returned a {} error code!",
                    error.status().unwrap()
                );
                if error.is_server_error() {
                    emergency_exit("If this code is 503, please contact the developer (McSib) and report this to him.");
                } else {
                    println!("If error code is 4xx, this is a client side error.");
                    emergency_exit("Please contact the developer (McSib) about this problem if it is 403, 404, 421");
                }

                // This should never be called.
                bail!(format_err!("Failed to download image!"))
            }
        };
        let mut image_bytes: Vec<u8> = vec![];
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

    /// Processes vec and downloads all posts from it.
    fn download_posts_from_vec(
        &mut self,
        mut name: String,
        posts: &Vec<PostEntry>,
    ) -> Result<(), Error> {
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
            let (file_name, bytes) = self.download_post(&post.file_url)?;
            self.save_image(&mut name, &file_name, &bytes)?;
            progress_bar.inc();
        }

        progress_bar.finish_println("");
        Ok(())
    }

    /// Downloads all posts from collection.
    pub fn download_posts_from_collection(&mut self, collection: Collection) -> Result<(), Error> {
        let single_posts = &collection.single_posts;
        self.download_posts_from_vec(single_posts.name.clone(), &single_posts.posts)?;

        for named_post in &collection.named_posts {
            self.download_posts_from_vec(named_post.name.clone(), &named_post.posts)?;
        }

        Ok(())
    }
}
