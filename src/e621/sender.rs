extern crate failure;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Read;
use std::rc::Rc;
use std::time::Duration;

use failure::Error;
use reqwest::header::USER_AGENT;
use reqwest::{Client, RequestBuilder, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// If an error occurs from server, it will respond with this.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorEntry {
    /// If the attempted grab is a success.
    pub success: bool,
    /// Error message of failed grab if `success` is false.
    pub msg: String,
}

/// Time the post was created.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeSet {
    pub json_class: String,
    /// Time in seconds.
    pub s: i64,
    /// Time in nano-seconds.
    pub n: i64,
}

/// Alias tag with id linking to the tag it was aliased to.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AliasEntry {
    /// ID of the alias.
    pub id: i64,
    /// Name of the alias tag.
    pub name: String,
    /// ID of the tag that the alias is tied to.
    pub alias_id: i64,
    /// Approval status of the alias tag.
    pub pending: bool,
}

/// GET return for set entry on e621/e926.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetEntry {
    /// ID of the set.
    pub id: i64,
    /// Name of the set.
    pub name: String,
    /// Time the set was created.
    pub created_at: TimeSet,
    /// Time the set was last updated.
    pub updated_at: TimeSet,
    /// ID of the user who created the set and updates it.
    pub user_id: i64,
    /// Description of the set.
    pub description: String,
    /// The short name of the set.
    #[serde(rename = "shortname")]
    pub short_name: String,
    /// The amount of posts contained in the set.
    pub post_count: i64,
    /// IDs for all posts in the set.
    pub posts: Vec<i64>,
}

/// GET return for post entry on e621/e926.
#[derive(Deserialize, Clone, Debug)]
pub struct TagEntry {
    /// Id of the tag.
    pub id: u32,
    /// Name of the tag.
    pub name: String,
    /// Number of all posts that use this tag.
    pub count: u32,
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
    #[serde(rename = "type")]
    pub tag_type: u8,
    /// If the type is locked (this value can also be `None` if not explicitly set by the admins).
    pub type_locked: Option<bool>,
}

/// GET return for post entry on e621/e926.
///
/// # Important
///
/// If the post that is loaded happens to be deleted when loaded, these properties will not be usable:
/// `source`, `sources`, `md5`, `file_size`, `file_ext`, `preview_width`, `preview_height`, `sample_url`, `sample_width`, `sample_height`, `has_children`, `children`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostEntry {
    /// The ID of the post.
    pub id: i64,
    /// Tags from the post.
    pub tags: String,
    /// Tags that are locked by the admins.
    ///
    /// # Important
    /// Can be `None` if there are no tags.
    pub locked_tags: Option<String>,
    /// Description of the post.
    pub description: String,
    /// When the post was uploaded.
    pub created_at: TimeSet,
    /// User ID of the user who uploaded the post.
    pub creator_id: Option<i64>,
    /// Username of the user who uploaded the post.
    pub author: String,
    /// The amount of changes that the post went through since uploaded.
    pub change: i64,
    /// The main source of the work, use `sources` instead when using all sources listed on post.
    ///
    /// # Important
    /// Can be `None` if no sources are given.
    pub source: Option<String>,
    /// How many upvoted or downvoted the post.
    pub score: i64,
    /// How many favorites the post has.
    pub fav_count: i64,
    /// The MD5 certification of the post.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub md5: Option<String>,
    /// Size of the source file.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub file_size: Option<i64>,
    /// URL of the source file.
    pub file_url: String,
    /// Extension of the source file (png, jpg, webm, gif, etc).
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub file_ext: Option<String>,
    /// URL for the preview file.
    pub preview_url: String,
    /// Width of the preview file.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub preview_width: Option<i64>,
    /// Height of the preview file.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub preview_height: Option<i64>,
    /// URL for the sample file.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub sample_url: Option<String>,
    /// Width of the sample file.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub sample_width: Option<i64>,
    /// Height of the sample file.
    ///
    /// # Important
    /// Can be `None` if the post is deleted or taken down.
    pub sample_height: Option<i64>,
    /// Rating of the post (safe, questionable, explicit), this will be "s", "q", "e".
    pub rating: String,
    /// Post status, one of: active, flagged, pending, deleted.
    pub status: String,
    /// Width of image.
    pub width: i64,
    /// Height of image.
    pub height: i64,
    /// If the post has comments.
    pub has_comments: bool,
    /// If the post has notes.
    pub has_notes: bool,
    /// If the post has children.
    ///
    /// # Important
    /// Can be `None` if the post has no children.
    pub has_children: Option<bool>,
    /// All of the children attached to post.
    ///
    /// # Important
    /// Can be `None` if the post has no children.
    pub children: Option<String>,
    /// If this post is a child, this will be the parent post's ID.
    ///
    /// # Important
    /// Can be `None` if the post has no parent.
    pub parent_id: Option<i64>,
    /// The artist or artists that drew this image.
    pub artist: Vec<String>,
    /// All the sources for the work.
    ///
    /// # Important
    /// Can be `None` if no sources are supplied to the post.
    pub sources: Option<Vec<String>>,
}

/// GET return for pool entry on e621/e926.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PoolEntry {
    /// Time the pool was created.
    pub created_at: TimeSet,
    /// Description of the pool.
    pub description: String,
    /// Id of the pool.
    pub id: i64,
    /// If the pool is active or not.
    pub is_active: bool,
    /// If the pool is locked or not.
    pub is_locked: bool,
    /// Name of the pool.
    pub name: String,
    /// The amount of posts added to the pool.
    pub post_count: i64,
    /// Time the pool was updated.
    pub updated_at: TimeSet,
    /// Id of user who created and updated post.
    pub user_id: i64,
    /// All posts in the pool.
    pub posts: Vec<PostEntry>,
}

/// Default user agent value.
static USER_AGENT_VALUE: &str = "e621_downloader/1.5.3 (by McSib on e621)";

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
            .cookie_store(false)
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

impl RequestSender {
    pub fn new() -> Self {
        RequestSender {
            client: SenderClient::new(),
            urls: Rc::new(RefCell::new(RequestSender::initialize_url_map())),
        }
    }

    fn initialize_url_map() -> HashMap<String, String> {
        hashmap![
            ("posts", "https://e621.net/post/index.json"),
            ("pool", "https://e621.net/pool/show.json"),
            ("set", "https://e621.net/set/show.json"),
            ("single", "https://e621.net/post/show.json"),
            ("blacklist", "https://e621.net/user/blacklist.json"),
            ("tag", "https://e621.net/tag/show.json"),
            ("tag_bulk", "https://e621.net/tag/index.json"),
            ("alias", "https://e621.net/tag_alias/index.json")
        ]
    }

    pub fn update_to_safe(&mut self) {
        self.urls
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, value)| *value = value.replace("e621", "e926"));
    }

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

    pub fn check_result(&self, result: Result<Response, reqwest::Error>) -> Response {
        match result {
            Ok(response) => response,
            Err(ref error) => {
                self.output_error(error);
                unreachable!()
            }
        }
    }

    pub fn get_entry_from_id<T>(&self, id: &str, url_type_key: &str) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let entry: T = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()[url_type_key])
                    .query(&[("id", id)])
                    .send(),
            )
            .json()?;

        Ok(entry)
    }

    /// Sends request to download image.
    pub fn download_image(&self, url: &str, file_size: i64) -> Result<Vec<u8>, Error> {
        let mut image_response = self.check_result(self.client.get(url).send());
        let mut image_bytes: Vec<u8> = Vec::with_capacity(file_size as usize);
        image_response.read_to_end(&mut image_bytes)?;

        Ok(image_bytes)
    }

    pub fn bulk_search(&self, searching_tag: &str, page: u16) -> Result<Vec<PostEntry>, Error> {
        let searched_posts: Vec<PostEntry> = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()["posts"])
                    .query(&[
                        ("tags", searching_tag),
                        ("page", &format!("{}", page)),
                        ("limit", &format!("{}", 320)),
                    ])
                    .send(),
            )
            .json()?;
        Ok(searched_posts)
    }

    pub fn get_tags_by_name(&self, tag: &str) -> Result<Vec<TagEntry>, Error> {
        let tags: Vec<TagEntry> = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()["tag_bulk"])
                    .query(&[("name", tag)])
                    .send(),
            )
            .json()?;
        Ok(tags)
    }

    pub fn get_tag_by_id(&self, id: &str) -> Result<TagEntry, Error> {
        let tag: TagEntry = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()["tag"])
                    .query(&[("id", id)])
                    .send(),
            )
            .json()?;
        Ok(tag)
    }

    pub fn query_aliases(&self, tag: &str) -> Result<Vec<AliasEntry>, Error> {
        let alias = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()["alias"])
                    .query(&[("query", tag)])
                    .send(),
            )
            .json()?;
        Ok(alias)
    }

    pub fn get_blacklist(&self, login: &Login) -> Result<Value, Error> {
        let blacklist = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()["blacklist"])
                    .query(&[
                        ("login", login.username.as_str()),
                        ("password_hash", login.password_hash.as_str()),
                    ])
                    .send(),
            )
            .json()?;
        Ok(blacklist)
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
