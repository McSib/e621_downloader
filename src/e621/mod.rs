extern crate chrono;
extern crate failure;
extern crate reqwest;
extern crate serde;

use std::collections::HashMap;
use std::io::stdin;

use chrono::Local;
use failure::Error;
use reqwest::{header::USER_AGENT, Client, RequestBuilder};
use serde::Serialize;

use crate::e621::data_sets::{PoolEntry, PostEntry, SetEntry};
use crate::e621::io::tag::{Group, Parsed, Tag};
use crate::e621::io::Config;

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
#[derive(Default)]
struct GrabbedPosts {
    /// All posts from pools
    pools: Vec<NamedPost>,
    /// All posts from sets
    sets: Vec<NamedPost>,
    /// All individual posts
    singles: Vec<PostEntry>,
    /// All posts under a searching tag
    posts: Vec<NamedPost>,
}

/// A collection of `Vec<NamedPost>` and `Vec<PostEntry>`.
#[derive(Default, Debug)]
struct Collection {
    /// All named posts
    named_posts: Vec<NamedPost>,
    /// All individual posts
    single_posts: Vec<PostEntry>,
}

impl Collection {
    /// Processes all collected posts from a search and appends them to `self.named_posts` and `self.single_posts`.
    fn set_posts(&mut self, collected_posts: &mut GrabbedPosts) {
        self.named_posts.append(&mut collected_posts.pools);
        self.named_posts.append(&mut collected_posts.sets);
        self.single_posts.append(&mut collected_posts.singles);
        self.named_posts.append(&mut collected_posts.posts);
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
                        self.grabbed_posts.singles.push(entry);

                        println!("\"{}\" post grabbed!", id);
                    }
                    Parsed::General(tag) => {
                        let name = match tag {
                            Tag::General(tag) => tag.clone(),
                            Tag::Special(tag) => {
                                let mut name = tag.clone();
                                self.update_tag_date(name.as_str());
                                self.add_date_to_tag(&mut name);
                                name
                            }
                            Tag::None => String::new(),
                        };
                        let posts = self.get_posts_from_tag(&tag_client, tag)?;
                        self.grabbed_posts
                            .posts
                            .push(NamedPost::from(&(name.as_str(), posts.as_slice())));
                        println!("\"{}\" grabbed!", name);
                    }
                };
            }
        }

        Ok(())
    }

    fn get_posts_from_tag(&self, client: &Client, tag: &Tag) -> Result<Vec<PostEntry>, Error> {
        match tag {
            Tag::General(tag_search) => {
                let limit: u8 = 5;
                let mut posts: Vec<PostEntry> = vec![];
                for page in 1..limit {
                    posts.append(
                        &mut self
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
                            .json::<Vec<PostEntry>>()?,
                    );
                }

                Ok(posts)
            }
            Tag::Special(tag_search) => {
                let mut page: u16 = 1;
                let mut posts = vec![];
                loop {
                    let mut data_set: Vec<PostEntry> = self
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
        let client_builder = Client::builder();
        let client = client_builder.cookie_store(false).tcp_nodelay().build()?;

        let name = set.name.as_str();
        let mut posts: Vec<PostEntry> = vec![];
        for id in &set.posts {
            posts.push(
                self.get_request_builder(&client, "single", &[("id", id)])
                    .send()?
                    .json()?,
            );
        }

        Ok(NamedPost::from(&(name, posts.as_slice())))
    }
}

pub struct EsixWebConnector<'a> {
    /// All urls that can be used.
    /// These options are `"post"`, `"pool"`, `"set"`, and `"single"`
    urls: HashMap<String, String>,
    /// The config which is modified when grabbing posts
    config: &'a mut Config,
    //    client: Client,
    /// Collection of all posts grabbed and posts to be downloaded
    collection: Collection,
}

impl<'a> EsixWebConnector<'a> {
    /// Initializes urls for the `Hashmap<String, String>`.
    fn initialize_urls(urls: &mut HashMap<String, String>) {
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

    /// Creates instance of `Self` for grabbing and downloading posts.
    pub fn new(config: &'a mut Config) -> Self {
        let mut connector = EsixWebConnector {
            urls: HashMap::new(),
            config,
            //            client: Client::new(),
            collection: Collection::default(),
        };

        EsixWebConnector::initialize_urls(&mut connector.urls);

        connector
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
    pub fn grab_posts(&mut self, groups: &[Group]) -> Result<(), Error> {
        let mut post_grabber = Grabber::from_tags(groups, &self.urls, self.config)?;
        self.collection.set_posts(&mut post_grabber.grabbed_posts);

        Ok(())
    }
}
