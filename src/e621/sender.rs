extern crate reqwest;
extern crate serde;
extern crate serde_json;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::USER_AGENT;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::e621::io::tag::TagType;
use crate::e621::io::{emergency_exit, Login};

/// A simple hack to create a `HashMap` using tuples. This macro is similar to the example of the simplified `vec!` macro in its structure and usage.
#[macro_export]
macro_rules! hashmap {
    ( $( $x:expr ),* ) => {
        {
            let mut hash_map: HashMap<String, String> = HashMap::new();
            $(
                let (calling_name, url) = $x;
                hash_map.insert(String::from(calling_name), String::from(url));
            )*

            hash_map
        }
    };
}

pub trait ToTagType {
    /// Converts `self` to `TagType`.
    fn to_tag_type(&self) -> TagType;
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AliasEntry {
    pub id: i64,
    pub antecedent_name: String,
    pub reason: String,
    pub creator_id: i64,
    pub created_at: String,
    pub forum_post_id: Option<i64>,
    pub updated_at: Option<String>,
    pub forum_topic_id: Option<i64>,
    pub consequent_name: String,
    pub status: String,
    pub post_count: i64,
    pub approver_id: Option<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetEntry {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub creator_id: i64,
    pub is_public: bool,
    pub name: String,
    pub shortname: String,
    pub description: String,
    pub post_count: i64,
    pub transfer_on_delete: bool,
    pub post_ids: Vec<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagEntry {
    pub id: i64,
    pub name: String,
    pub post_count: i64,
    pub related_tags: String,
    pub related_tags_updated_at: String,
    /// The type of tag it is.
    ///
    /// # Important
    /// This tag can be the following types:
    /// `0`: General;
    /// `1`: Artist;
    /// `2`: Nil (This used to be something, but was removed);
    /// `3`: Copyright;
    /// `4`: Character;
    /// `5`: Species;
    pub category: u8,
    pub is_locked: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl ToTagType for TagEntry {
    /// Constrains the `TagType` enum to a tags type specifically.
    /// This can only be `TagType::General` or `TagType::Artist`.
    fn to_tag_type(&self) -> TagType {
        match self.category {
            // `0`: General; `3`: Copyright; `5`: Species;
            0 | 3 | 5 => TagType::General,
            // `4`: Character;
            4 => TagType::General,
            // `1`: Artist;
            1 => TagType::Artist,
            _ => unreachable!(),
        }
    }
}

/// GET return for post entry on e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostEntry {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub file: File,
    pub preview: Preview,
    pub sample: Sample,
    pub score: Score,
    pub tags: Tags,
    pub locked_tags: Vec<String>,
    pub change_seq: i64,
    pub flags: Flags,
    pub rating: String,
    pub fav_count: i64,
    pub sources: Vec<String>,
    pub pools: Vec<i64>,
    pub relationships: Relationships,
    pub approver_id: Option<i64>,
    pub uploader_id: i64,
    pub description: String,
    pub comment_count: i64,
    pub is_favorited: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    pub width: i64,
    pub height: i64,
    pub ext: String,
    pub size: i64,
    pub md5: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub width: i64,
    pub height: i64,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub has: Option<bool>,
    pub height: i64,
    pub width: i64,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub up: i64,
    pub down: i64,
    pub total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tags {
    pub general: Vec<String>,
    pub species: Vec<String>,
    pub character: Vec<String>,
    pub copyright: Vec<String>,
    pub artist: Vec<String>,
    pub invalid: Vec<String>,
    pub lore: Vec<String>,
    pub meta: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flags {
    pub pending: bool,
    pub flagged: bool,
    pub note_locked: bool,
    pub status_locked: bool,
    pub rating_locked: bool,
    pub deleted: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationships {
    pub parent_id: Option<i64>,
    pub has_children: bool,
    pub has_active_children: bool,
    pub children: Vec<i64>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolEntry {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub creator_id: i64,
    pub description: String,
    pub is_active: bool,
    pub category: String,
    pub is_deleted: bool,
    pub post_ids: Vec<i64>,
    pub creator_name: String,
    pub post_count: i64,
}

/// Default user agent value.
const USER_AGENT_VALUE: &str = "e621_downloader/1.5.6 (by McSib on e621)";

/// Sender client is a modified form of the generic client, wrapping the client in a `Rc` so the sender client can be cloned without creating another instance of the root client.
struct SenderClient {
    /// `Client` wrapped in a `Rc` so only one instance of the client exists. This will prevent an overabundance of clients in the code.
    client: Rc<Client>,
}

impl SenderClient {
    /// Creates root client for the `SenderClient`.
    fn new() -> Self {
        SenderClient {
            client: Rc::new(SenderClient::build_client()),
        }
    }

    /// Runs client through a builder to give it required settings.
    /// Cookies aren't stored in the client, TCP_NODELAY is on, and timeout is changed from 30 seconds to 60.
    fn build_client() -> Client {
        Client::builder()
            .tcp_nodelay()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    /// A wrapping function that acts the exact same as `self.client.get` but will instead attach the user agent header before returning the `RequestBuilder`.
    /// This will ensure that all requests sent have the proper user agent info.
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url).header(USER_AGENT, USER_AGENT_VALUE)
    }
}

impl Clone for SenderClient {
    /// Creates a new instance of SenderClient, but clones the `Rc` of the root client, ensuring that all requests are going to the same client.
    fn clone(&self) -> Self {
        SenderClient {
            client: self.client.clone(),
        }
    }
}

/// The `RequestSender`, it handles all calls to the API, so every single instance in the program must adhere to the `RequestSender`.
pub struct RequestSender {
    /// The client that will be used to send all requests.
    ///
    /// # Important
    /// Even though the `SenderClient` isn't wrapped in a `Rc`, the main client inside of it is, this will ensure that all request are only sent through one client.
    client: SenderClient,
    urls: Rc<RefCell<HashMap<String, String>>>,
}

// TODO: All API calls here need to be rewritten for the new API, none of these requests will work.
impl RequestSender {
    pub fn new() -> Self {
        RequestSender {
            client: SenderClient::new(),
            urls: Rc::new(RefCell::new(RequestSender::initialize_url_map())),
        }
    }

    /// Initialized all the urls that will be used by the sender.
    fn initialize_url_map() -> HashMap<String, String> {
        // TODO: Urls need to be updated to reflect the new API calls
        hashmap![
            ("posts", "https://e621.net/posts.json"),
            ("pool", "https://e621.net/pools.json"),
            ("set", "https://e621.net/post_sets.json"),
            // TODO: Single posts requires the proper ID to be appended to the url.
            ("single", "https://e621.net/posts/"),
            // TODO: The blacklist url requires the user to be logged in, as well as there ID being supplied.
            ("blacklist", "https://e621.net/users/"),
            // TODO: Single tags requires the proper ID to be appended to the url.
            ("tag", "https://e621.net/tags/"),
            ("tag_bulk", "https://e621.net/tags.json"),
            ("alias", "https://e621.net/tag_aliases.json")
        ]
    }

    /// Updates all the urls from e621 to e926.
    pub fn update_to_safe(&mut self) {
        self.urls
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, value)| *value = value.replace("e621", "e926"));
    }

    /// If a request failed, this will output what type of error it is before exiting.
    fn output_error(&self, error: &reqwest::Error) {
        eprintln!(
            "Error occurred from sent request. \
             Error: {}",
            error
        );
        eprintln!("Url where error occurred: {:#?}", error.url());

        if let Some(status) = error.status() {
            let code = status.as_u16();
            eprintln!("The response code from the server was: {}", code);

            match code {
                500 => {
                    eprintln!(
                        "There was an error that happened internally in the servers, \
                         please try using the downloader later until the issue is solved."
                    );
                }
                503 => {
                    eprintln!(
                        "Server could not handle the request, or the downloader has \
                         exceeded the rate-limit. Contact the developer immediately about this \
                         issue."
                    );
                }
                403 => {
                    eprintln!(
                        "The client was forbidden from accessing the api, contact the \
                         developer immediately if this error occurs."
                    );
                }
                421 => {
                    eprintln!(
                        "The user is throttled, thus the request is unsuccessful. \
                         Contact the developer immediately if this error occurs."
                    );
                }
                _ => {
                    eprintln!("Response code couldn't be posted...");
                }
            }
        }

        emergency_exit("To prevent the program from crashing, it will do an emergency exit.");
    }

    /// Gets the response from a sent request and checks to ensure it was successful.
    pub fn check_result(&self, result: Result<Response, reqwest::Error>) -> Response {
        match result {
            Ok(response) => response,
            Err(ref error) => {
                self.output_error(error);
                unreachable!()
            }
        }
    }

    /// Sends request to download image.
    pub fn download_image(&self, url: &str, file_size: i64) -> Vec<u8> {
        let mut image_response = self.check_result(self.client.get(url).send());
        let mut image_bytes: Vec<u8> = Vec::with_capacity(file_size as usize);
        image_response
            .copy_to(&mut image_bytes)
            .expect("Failed to download image!");

        image_bytes
    }

    /// Gets an entry of `T` by their ID and returns it.
    pub fn get_entry_from_id<T>(&self, id: &str, url_type_key: &str) -> T
    where
        T: DeserializeOwned,
    {
        self.check_result(
            self.client
                .get(&self.urls.borrow()[url_type_key])
                .query(&[("id", id)])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to entry!")
    }

    /// Get a single pool entry by ID and grabbing a page of posts from it.
    pub fn get_pool_entry(&self, id: &str, page: u16) -> PoolEntry {
        self.check_result(
            self.client
                .get(&self.urls.borrow()["pool"])
                .query(&[("id", id), ("page", &page.to_string())])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to PoolEntry!")
    }

    /// Performs a bulk search for posts using tags to filter the response.
    pub fn bulk_search(&self, searching_tag: &str, page: u16) -> Vec<PostEntry> {
        self.check_result(
            self.client
                .get(&self.urls.borrow()["posts"])
                .query(&[
                    ("tags", searching_tag),
                    ("page", &format!("{}", page)),
                    ("limit", &format!("{}", 320)),
                ])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to Vec<PostEntry>!")
    }

    /// Gets tags by their name.
    pub fn get_tags_by_name(&self, tag: &str) -> Vec<TagEntry> {
        self.check_result(
            self.client
                .get(&self.urls.borrow()["tag_bulk"])
                .query(&[("name", tag)])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to Vec<TagEntry>!")
    }

    /// Gets tags by their ID.
    pub fn get_tag_by_id(&self, id: &str) -> TagEntry {
        self.check_result(
            self.client
                .get(&self.urls.borrow()["tag"])
                .query(&[("id", id)])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to TagEntry!")
    }

    /// Queries aliases and returns response.
    pub fn query_aliases(&self, tag: &str) -> Vec<AliasEntry> {
        self.check_result(
            self.client
                .get(&self.urls.borrow()["alias"])
                .query(&[("query", tag)])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to Vec<AliasEntry>!")
    }

    /// Gets the blacklist and returns the value.
    pub fn get_blacklist(&self, login: &Login) -> Value {
        self.check_result(
            self.client
                .get(&self.urls.borrow()["blacklist"])
                .query(&[
                    ("login", login.username.as_str()),
                    ("password_hash", login.password_hash.as_str()),
                ])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to Value!")
    }
}

impl Clone for RequestSender {
    fn clone(&self) -> Self {
        RequestSender {
            client: self.client.clone(),
            urls: self.urls.clone(),
        }
    }
}
