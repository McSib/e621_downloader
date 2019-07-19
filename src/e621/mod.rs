extern crate chrono;
extern crate failure;
extern crate reqwest;
extern crate serde;

use std::collections::HashMap;
use std::io::{stdin, Read, Write};

use chrono::Local;
use failure::Error;
use reqwest::{header::USER_AGENT, Client, RequestBuilder};
use serde::Serialize;

use self::reqwest::Url;
use crate::e621::data_sets::{PoolEntry, PostEntry, SetEntry};
use crate::e621::io::tag::{Group, Parsed, Tag};
use crate::e621::io::Config;
use pbr::ProgressBar;
use std::fs::{create_dir_all, File};
use std::path::Path;

mod data_sets;
pub mod io;

/// Default user agent value.
static USER_AGENT_VALUE: &'static str = "e621_downloader/0.0.1 (by McSib on e621)";

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
    pub fn new(name: String) -> Self {
        NamedPost {
            name,
            posts: vec![],
        }
    }
}

impl Default for NamedPost {
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

    fn get_posts_from_tag(&mut self, client: &Client, tag: &Tag) -> Result<Vec<PostEntry>, Error> {
        match tag {
            Tag::General(tag_search) => {
                let limit: u8 = 5;
                let mut posts: Vec<PostEntry> = vec![];
                for page in 1..limit {
                    let mut searched_posts: Vec<PostEntry> = self
                        .get_request_builder(
                            &client,
                            "post",
                            &[
                                ("tags", tag_search),
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
            Tag::Special(tag_search) => {
                let mut tag_search = tag_search.clone();
                self.update_tag_date(tag_search.as_str());
                self.add_date_to_tag(&mut tag_search);

                let mut page: u16 = 1;
                let mut posts = vec![];
                loop {
                    let mut data_set: Vec<PostEntry> = self
                        .get_request_builder(
                            &client,
                            "post",
                            &[
                                ("tags", &tag_search),
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
            Tag::None => bail!(format_err!("The tag is none!")),
        }
    }

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
    //    /// Collection of all posts grabbed and posts to be downloaded
    //    collection: Collection,
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
            //            collection: Collection::default(),
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

        let file_dir = format!("{}{}", self.config.download_directory, dir_name);
        let dir = Path::new(file_dir.as_str());
        if !dir.exists() {
            create_dir_all(dir)?;
        }

        let mut image_file: File = File::create(dir.join(file_name))?;
        image_file.write_all(bytes.as_slice())?;

        Ok(())
    }

    fn download_post(&self, url: &String) -> Result<(String, Vec<u8>), Error> {
        let mut image_response = self
            .client
            .get(url)
            .header(USER_AGENT, USER_AGENT_VALUE)
            .send()?;
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

    pub fn download_posts_from_collection(&mut self, collection: Collection) -> Result<(), Error> {
        let single_posts = &collection.single_posts;
        self.download_posts_from_vec(single_posts.name.clone(), &single_posts.posts)?;

        for named_post in &collection.named_posts {
            self.download_posts_from_vec(named_post.name.clone(), &named_post.posts)?;
        }

        Ok(())
    }

    pub fn download_posts(&mut self, collection: Collection) -> Result<(), Error> {
        self.download_posts_from_collection(collection)?;

        Ok(())
    }
}
