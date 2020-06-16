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

use self::reqwest::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use self::serde_json::{from_value, to_string, Value};
use crate::e621::io::tag::TagType;
use crate::e621::io::{emergency_exit, Login};
use std::fs::write;

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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulkPostEntry {
    pub posts: Vec<PostEntry>,
}

/// GET return for post entry on e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostEntry {
    pub id: i64,
    pub created_at: String,
    pub updated_at: Option<String>,
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
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    pub width: i64,
    pub height: i64,
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    pub has: Option<bool>,
    pub height: i64,
    pub width: i64,
    pub url: Option<String>,
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

impl Tags {
    pub fn combine_tags(&mut self) -> Vec<String> {
        let vecs: Vec<&mut Vec<String>> = vec![
            &mut self.general,
            &mut self.species,
            &mut self.character,
            &mut self.copyright,
            &mut self.artist,
            &mut self.invalid,
            &mut self.lore,
            &mut self.meta,
        ];
        let capacity = Tags::get_total_tags_len(&vecs);
        let mut tags: Vec<String> = Vec::with_capacity(capacity);
        Tags::append_all_vecs(&mut tags, vecs);

        tags
    }

    fn get_total_tags_len<T>(vecs: &[&mut Vec<T>]) -> usize {
        let mut len: usize = 0;
        for vec in vecs {
            len += vec.len();
        }

        len
    }

    fn append_all_vecs<T>(dest: &mut Vec<T>, src: Vec<&mut Vec<T>>) {
        for vec in src {
            dest.append(vec);
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flags {
    pub pending: bool,
    pub flagged: bool,
    pub note_locked: bool,
    pub status_locked: Option<bool>,
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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserEntry {
    pub wiki_page_version_count: i64,
    pub artist_version_count: i64,
    pub pool_version_count: i64,
    pub forum_post_count: i64,
    pub comment_count: i64,
    pub appeal_count: i64,
    pub flag_count: i64,
    pub positive_feedback_count: i64,
    pub neutral_feedback_count: i64,
    pub negative_feedback_count: i64,
    pub upload_limit: i64,
    pub id: i64,
    pub created_at: String,
    pub name: String,
    pub level: i64,
    pub base_upload_limit: i64,
    pub post_upload_count: i64,
    pub post_update_count: i64,
    pub note_update_count: i64,
    pub is_banned: bool,
    pub can_approve_posts: bool,
    pub can_upload_free: bool,
    pub level_string: String,
    pub show_avatars: Option<bool>,
    pub blacklist_avatars: Option<bool>,
    pub blacklist_users: Option<bool>,
    pub description_collapsed_initially: Option<bool>,
    pub hide_comments: Option<bool>,
    pub show_hidden_comments: Option<bool>,
    pub show_post_statistics: Option<bool>,
    pub has_mail: Option<bool>,
    pub receive_email_notifications: Option<bool>,
    pub enable_keyboard_navigation: Option<bool>,
    pub enable_privacy_mode: Option<bool>,
    pub style_usernames: Option<bool>,
    pub enable_auto_complete: Option<bool>,
    pub has_saved_searches: Option<bool>,
    pub disable_cropped_thumbnails: Option<bool>,
    pub disable_mobile_gestures: Option<bool>,
    pub enable_safe_mode: Option<bool>,
    pub disable_responsive_mode: Option<bool>,
    pub disable_post_tooltips: Option<bool>,
    pub no_flagging: Option<bool>,
    pub no_feedback: Option<bool>,
    pub disable_user_dmails: Option<bool>,
    pub enable_compact_uploader: Option<bool>,
    pub updated_at: Option<String>,
    pub email: Option<String>,
    pub last_logged_in_at: Option<String>,
    pub last_forum_read_at: Option<String>,
    pub recent_tags: Option<String>,
    pub comment_threshold: Option<i64>,
    pub default_image_size: Option<String>,
    pub favorite_tags: Option<String>,
    pub blacklisted_tags: Option<String>,
    pub time_zone: Option<String>,
    pub per_page: Option<i64>,
    pub custom_style: Option<String>,
    pub favorite_count: Option<i64>,
    pub api_regen_multiplier: Option<i64>,
    pub api_burst_limit: Option<i64>,
    pub remaining_api_limit: Option<i64>,
    pub statement_timeout: Option<i64>,
    pub favorite_limit: Option<i64>,
    pub tag_query_limit: Option<i64>,
}

/// Default user agent value.
const USER_AGENT_VALUE: &str = "e621_downloader/1.6.0 (by McSib on e621)";

/// Sender client is a modified form of the generic client, wrapping the client in a `Rc` so the sender client can be cloned without creating another instance of the root client.
struct SenderClient {
    /// `Client` wrapped in a `Rc` so only one instance of the client exists. This will prevent an overabundance of clients in the code.
    client: Rc<Client>,
    auth: Rc<String>,
}

impl SenderClient {
    /// Creates root client for the `SenderClient`.
    fn new(auth: String) -> Self {
        SenderClient {
            client: Rc::new(SenderClient::build_client()),
            auth: Rc::new(auth),
        }
    }

    /// Runs client through a builder to give it required settings.
    /// Cookies aren't stored in the client, TCP_NODELAY is on, and timeout is changed from 30 seconds to 60.
    fn build_client() -> Client {
        Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    /// A wrapping function that acts the exact same as `self.client.get` but will instead attach the user agent header before returning the `RequestBuilder`.
    /// This will ensure that all requests sent have the proper user agent info.
    pub fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url).header(USER_AGENT, USER_AGENT_VALUE)
    }

    /// This is the same as `self.get(url)` but will attach the authorization header with username and API hash.
    pub fn get_with_auth(&self, url: &str) -> RequestBuilder {
        if self.auth.is_empty() {
            self.get(url)
        } else {
            self.get(url).header(AUTHORIZATION, self.auth.as_str())
        }
    }
}

impl Clone for SenderClient {
    /// Creates a new instance of SenderClient, but clones the `Rc` of the root client, ensuring that all requests are going to the same client.
    fn clone(&self) -> Self {
        SenderClient {
            client: self.client.clone(),
            auth: self.auth.clone(),
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
    pub fn new(login: &Login) -> Self {
        let auth = if login.is_empty() {
            String::new()
        } else {
            base64_url::encode(format!("{}:{}", login.username, login.api_key).as_str())
        };

        RequestSender {
            client: SenderClient::new(auth),
            urls: Rc::new(RefCell::new(RequestSender::initialize_url_map())),
        }
    }

    /// Initialized all the urls that will be used by the sender.
    fn initialize_url_map() -> HashMap<String, String> {
        hashmap![
            ("posts", "https://e621.net/posts.json"),
            ("pool", "https://e621.net/pools/"),
            ("set", "https://e621.net/post_sets/"),
            ("single", "https://e621.net/posts/"),
            ("blacklist", "https://e621.net/users/"),
            ("tag", "https://e621.net/tags/"),
            ("tag_bulk", "https://e621.net/tags.json"),
            ("alias", "https://e621.net/tag_aliases.json"),
            ("user", "https://e621.net/users/")
        ]
    }

    pub fn is_authenticated(&self) -> bool {
        !self.client.auth.is_empty()
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

    pub fn append_url(&self, url: &str, append: &str) -> String {
        format!("{}{}.json", url, append)
    }

    #[deprecated(
        since = "1.5.6",
        note = "This uses old workarounds and loopholes in the old API to make lesser calls to it. \
        This no longer works with the new API."
    )]
    /// Gets an entry of `T` by their ID and returns it.
    pub fn get_entry_from_id<T>(&self, id: &str, url_type_key: &str) -> T
    where
        T: DeserializeOwned,
    {
        self.check_result(
            self.client
                .get(&self.urls.borrow()[url_type_key])
                .query(&[("search[id]", id)])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to entry!")
    }

    pub fn get_entry_from_appended_id<T>(&self, id: &str, url_type_key: &str) -> T
    where
        T: DeserializeOwned,
    {
        // let json: Value = self
        //     .check_result(
        //         self.client
        //             .get(&self.append_url(&self.urls.borrow()[url_type_key], id))
        //             .send(),
        //     )
        //     .json()
        //     .unwrap();
        // let result = if url_type_key == "single" {
        //     json.get("post").unwrap().clone()
        // } else {
        //     json
        // };
        // write("posts.json", to_string(&result).unwrap()).unwrap();

        let value: Value = self
            .check_result(
                self.client
                    .get_with_auth(&self.append_url(&self.urls.borrow()[url_type_key], id))
                    .send(),
            )
            .json()
            .expect("Json was unable to deserialize to entry!");
        if url_type_key == "single" {
            from_value(value.get("post").unwrap().clone())
                .expect("Json was unable to deserialize to entry!")
        } else {
            from_value(value).expect("Json was unable to deserialize to entry!")
        }
    }

    #[deprecated(
        since = "1.5.6",
        note = "This uses the old API to grab the pool and is no longer used for the new API"
    )]
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
    pub fn bulk_search(&self, searching_tag: &str, page: u16) -> BulkPostEntry {
        // let json: Value = self
        //     .check_result(
        //         self.client
        //             .get(&self.urls.borrow()["posts"])
        //             .query(&[
        //                 ("tags", searching_tag),
        //                 ("page", &format!("{}", page)),
        //                 ("limit", &format!("{}", 320)),
        //             ])
        //             .send(),
        //     )
        //     .json()
        //     .unwrap();
        // write("posts.json", to_string(&json).unwrap()).unwrap();
        self.check_result(
            self.client
                .get_with_auth(&self.urls.borrow()["posts"])
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
        let result: Value = self
            .check_result(
                self.client
                    .get(&self.urls.borrow()["tag_bulk"])
                    .query(&[("search[name]", tag)])
                    .send(),
            )
            .json()
            .expect("Json was unable to deserialize!");
        if result.is_object() {
            vec![]
        } else {
            from_value::<Vec<TagEntry>>(result)
                .expect("Json was unable to deserialize to Vec<TagEntry>!")
        }
    }

    #[deprecated(
        since = "1.5.6",
        note = "This code is no longer needed since the alias checker has to \
    send a general search to the api, rather than id specific."
    )]
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
                .query(&[("search[antecedent_name]", tag)])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to Vec<AliasEntry>!")
    }

    // FIXME: This function could do with some nice cleaning.
    /// Gets the blacklist and returns the value.
    pub fn get_blacklist(&self, login: &Login) -> UserEntry {
        self.check_result(
            self.client
                .get_with_auth(&self.append_url(&self.urls.borrow()["user"], &login.username))
                // .query(&[
                //     ("login", login.username.as_str()),
                //     ("api_key", login.password_hash.as_str()),
                // ])
                .send(),
        )
        .json()
        .expect("Json was unable to deserialize to User entry!")
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
