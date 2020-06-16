extern crate reqwest;
extern crate serde;
extern crate serde_json;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::AUTHORIZATION;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::{from_value, Value};

use crate::e621::io::{emergency_exit, Login};
use crate::e621::io::tag::TagType;

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

/// GET return of alias entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AliasEntry {
    /// Alias ID.
    pub id: i64,
    /// Alias name.
    pub antecedent_name: String,
    /// Reason for the alias.
    pub reason: String,
    /// ID of the creator of the alias.
    pub creator_id: i64,
    /// The date the alias was created.
    pub created_at: String,
    /// Forum post id tied to the request for the alias to be approved.
    pub forum_post_id: Option<i64>,
    /// The date for when the alias was updated.
    pub updated_at: Option<String>,
    /// Forum topic ID for the thread where the request for alias approval was created.
    pub forum_topic_id: Option<i64>,
    /// Original tag name.
    pub consequent_name: String,
    /// Current status of the alias.
    /// Can be `approved`, `active`, `pending`, `deleted`, `retired`, `processing`, and `queued`.
    ///
    /// # Error
    /// Optionally, there can also be an `error` prompt with the following format:
    /// `"error: cannot update a new record"`
    /// ## Reason for Error
    /// This is probably an internal error with the server, and while it is exceptionally rare,
    /// there is still a probability.
    pub status: String,
    /// The amount of post the aliased tag is tied to.
    pub post_count: i64,
    /// ID of the user that approved the alias.
    pub approver_id: Option<i64>,
}

/// GET return of tag entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagEntry {
    /// Id of the tag.
    pub id: i64,
    /// Name of the tag.
    pub name: String,
    /// Amount of posts that uses the tag.
    pub post_count: i64,
    /// Related tags that this tag is commonly paired with.
    pub related_tags: String,
    /// Most recent date the `related_tags` was updated.
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
    /// Whether or not the tag is locked.
    pub is_locked: bool,
    /// The date the tag was created.
    pub created_at: String,
    /// The date the tag was updated.
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

/// Wrapper struct that holds the return of bulk searches.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulkPostEntry {
    pub posts: Vec<PostEntry>,
}

/// GET return of post entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostEntry {
    /// The ID number of the post.
    pub id: i64,
    /// The time the post was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub created_at: String,
    ///  The time the post was last updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub updated_at: Option<String>,
    /// The main image of the post.
    pub file: File,
    /// The preview image of the post.
    pub preview: Preview,
    /// The sample image of the post.
    pub sample: Sample,
    /// The score of the post.
    pub score: Score,
    /// The tags tied to the post.
    pub tags: Tags,
    /// An array of tags that are locked on the post.
    pub locked_tags: Vec<String>,
    /// An ID that increases for every post alteration on E6 (explained below)
    ///
    /// # Explanation
    /// change_seq is a number that is increased every time a post is changed on the site.
    /// It gets updated whenever a post has any of these values change:
    ///
    /// - `tag_string`
    /// - `source`
    /// - `description`
    /// - `rating`
    /// - `md5`
    /// - `parent_id`
    /// - `approver_id`
    /// - `is_deleted`
    /// - `is_pending`
    /// - `is_flagged`
    /// - `is_rating_locked`
    /// - `is_pending`
    /// - `is_flagged`
    /// - `is_rating_locked`
    pub change_seq: i64,
    /// All the flags that could be raised on the post.
    pub flags: Flags,
    /// The post’s rating. Either `s`, `q` or `e`.
    pub rating: String,
    /// How many people have favorited the post.
    pub fav_count: i64,
    /// The source field of the post.
    pub sources: Vec<String>,
    /// An array of Pool IDs that the post is a part of.
    pub pools: Vec<i64>,
    /// The relationships of the post.
    pub relationships: Relationships,
    /// The ID of the user that approved the post, if available.
    pub approver_id: Option<i64>,
    /// The ID of the user that uploaded the post.
    pub uploader_id: i64,
    /// The post’s description.
    pub description: String,
    /// The count of comments on the post.
    pub comment_count: i64,
    /// If provided auth credentials, will return if the authenticated user has favorited the post or not.
    /// HTTP Basic Auth is recommended over `login` and `api_key` parameters in the URL.
    pub is_favorited: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    /// The width of the post.
    pub width: i64,
    /// The height of the post.
    pub height: i64,
    /// The file’s extension.
    pub ext: String,
    /// The size of the file in bytes.
    pub size: i64,
    /// The md5 of the file.
    pub md5: String,
    /// The URL where the file is hosted on E6
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preview {
    /// The width of the post preview.
    pub width: i64,
    /// The height of the post preview.
    pub height: i64,
    /// The URL where the preview file is hosted on E6
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    ///  If the post has a sample/thumbnail or not.
    pub has: Option<bool>,
    /// The width of the post sample.
    pub height: i64,
    /// The height of the post sample.
    pub width: i64,
    /// The URL where the sample file is hosted on E6.
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    /// The number of times voted up.
    pub up: i64,
    /// A negative number representing the number of times voted down.
    pub down: i64,
    /// The total score (up + down).
    pub total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tags {
    /// An array of all the `general` tags on the post.
    pub general: Vec<String>,
    /// An array of all the `species` tags on the post.
    pub species: Vec<String>,
    /// An array of all the `character` tags on the post.
    pub character: Vec<String>,
    /// An array of all the `copyright` tags on the post.
    pub copyright: Vec<String>,
    /// An array of all the `artist` tags on the post.
    pub artist: Vec<String>,
    /// An array of all the `invalid` tags on the post.
    pub invalid: Vec<String>,
    /// An array of all the `lore` tags on the post.
    pub lore: Vec<String>,
    /// An array of all the `meta` tags on the post.
    pub meta: Vec<String>,
}

impl Tags {
    /// Combines all of the tags into a single array.
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

    /// Get the total calculated length of all tag vecs as one.
    fn get_total_tags_len<T>(vecs: &[&mut Vec<T>]) -> usize {
        let mut len: usize = 0;
        for vec in vecs {
            len += vec.len();
        }

        len
    }

    /// Append a `Vec<Vec<T>>` to a `Vec<T>`.
    fn append_all_vecs<T>(dest: &mut Vec<T>, src: Vec<&mut Vec<T>>) {
        for vec in src {
            dest.append(vec);
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flags {
    /// If the post is pending approval.
    pub pending: bool,
    /// If the post is flagged for deletion.
    pub flagged: bool,
    /// If the post has it’s notes locked.
    pub note_locked: bool,
    /// If the post’s status has been locked.
    pub status_locked: Option<bool>,
    /// If the post’s rating has been locked.
    pub rating_locked: bool,
    /// If the post has been deleted.
    pub deleted: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationships {
    /// The ID of the post’s parent, if it has one.
    pub parent_id: Option<i64>,
    /// If the post has child posts.
    pub has_children: bool,
    pub has_active_children: bool,
    /// A list of child post IDs that are linked to the post, if it has any.
    pub children: Vec<i64>,
}

/// GET return of set entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetEntry {
    /// The ID of the set.
    pub id: i64,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub created_at: String,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub updated_at: String,
    /// The ID of the user that created the set.
    pub creator_id: i64,
    /// If the set is public and visible.
    pub is_public: bool,
    /// The name of the set.
    pub name: String,
    /// The short name of the set.
    pub shortname: String,
    /// The description of the set.
    pub description: String,
    /// The amount of posts in the set.
    pub post_count: i64,
    /// If the set will transfer its post on delete.
    pub transfer_on_delete: bool,
    /// An array group of posts in the pool.
    pub post_ids: Vec<i64>,
}

/// GET return of pool entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PoolEntry {
    /// The ID of the pool.
    pub id: i64,
    /// The name of the pool.
    pub name: String,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub created_at: String,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub updated_at: String,
    /// The ID of the user that created the pool.
    pub creator_id: i64,
    /// The description of the pool.
    pub description: String,
    /// If the pool is active and still getting posts added.
    pub is_active: bool,
    /// Can be `series` or `collection`.
    pub category: String,
    /// If the pool has been deleted.
    pub is_deleted: bool,
    /// An array group of posts in the pool.
    pub post_ids: Vec<i64>,
    /// The name of the user that created the pool.
    pub creator_name: String,
    /// The amount of posts in the pool.
    pub post_count: i64,
}

/// GET return of user entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserEntry {
    /// The amount of wiki changes made by the user.
    pub wiki_page_version_count: i64,
    /// The amount of artist changes made by the user.
    pub artist_version_count: i64,
    /// The amount of pool changes made by the user.
    pub pool_version_count: i64,
    /// The amount of post changes made by the user.
    pub forum_post_count: i64,
    /// Count of comments posted by the user.
    pub comment_count: i64,
    /// Count of appeals done by the user.
    pub appeal_count: i64,
    /// Count of flags done by the user.
    pub flag_count: i64,
    /// The amount of positive feedback given by the user.
    pub positive_feedback_count: i64,
    /// The amount of neutral feedback given by the user.
    pub neutral_feedback_count: i64,
    /// The amount of negative feedback given by the user.
    pub negative_feedback_count: i64,
    /// Upload limit of the user.
    pub upload_limit: i64,
    /// ID of the user.
    pub id: i64,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub created_at: String,
    /// Name of the user.
    pub name: String,
    /// Level of the user.
    pub level: i64,
    /// Base upload limit of the user.
    pub base_upload_limit: i64,
    /// Count of posts uploaded by the user.
    pub post_upload_count: i64,
    /// Count of posts updated by the user.
    pub post_update_count: i64,
    /// Count of notes updated by the user.
    pub note_update_count: i64,
    /// If user is banned or not.
    pub is_banned: bool,
    /// Whether or not the user can approve posts.
    pub can_approve_posts: bool,
    /// Whether or not uploading posts affect the post limit.
    pub can_upload_free: bool,
    /// The string of the user's current level.
    pub level_string: String,
    /// Whether or not avatars should be shown.
    pub show_avatars: Option<bool>,
    /// Whether or not the blacklist should block avatars.
    pub blacklist_avatars: Option<bool>,
    /// Whether or not the blacklist should block users.
    pub blacklist_users: Option<bool>,
    /// Whether or not a post's description should be collapsed initially.
    pub description_collapsed_initially: Option<bool>,
    /// Whether or not comments should be hidden.
    pub hide_comments: Option<bool>,
    /// Whether or not hidden comments should be shown.
    pub show_hidden_comments: Option<bool>,
    /// Whether or not to show post statistics.
    pub show_post_statistics: Option<bool>,
    /// Whether or not the user has mail.
    pub has_mail: Option<bool>,
    /// Whether or not the user will receive email notifications.
    pub receive_email_notifications: Option<bool>,
    /// Whether or not keyboard navigation is on/off.
    pub enable_keyboard_navigation: Option<bool>,
    /// Whether or not privacy mode is enabled.
    pub enable_privacy_mode: Option<bool>,
    /// Whether or not usernames should be styled.
    pub style_usernames: Option<bool>,
    /// Whether auto complete should be on or off.
    pub enable_auto_complete: Option<bool>,
    /// Whether or not searches should be saved.
    pub has_saved_searches: Option<bool>,
    /// Whether or not thumbnails should be cropped.  
    pub disable_cropped_thumbnails: Option<bool>,
    /// Whether or not mobile gestures should be on or off.
    pub disable_mobile_gestures: Option<bool>,
    /// Whether or not safe mode is on/off.
    pub enable_safe_mode: Option<bool>,
    /// Whether or not resposive mode is disabled.
    pub disable_responsive_mode: Option<bool>,
    /// Whether or not post tooltips is disabled.
    pub disable_post_tooltips: Option<bool>,
    /// Whether or not the user can't flag.
    pub no_flagging: Option<bool>,
    /// Whether or not the user can't give feedback.
    pub no_feedback: Option<bool>,
    /// Whether or not dmail is disabled.
    pub disable_user_dmails: Option<bool>,
    /// Whether or not compact uploader is enabled.
    pub enable_compact_uploader: Option<bool>,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub updated_at: Option<String>,
    /// The user's email.
    pub email: Option<String>,
    /// The time the user was last logged in in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub last_logged_in_at: Option<String>,
    /// The time the last forum the user read in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub last_forum_read_at: Option<String>,
    /// Recent tags searched by the user.
    pub recent_tags: Option<String>,
    /// Comment threshold of the user.
    pub comment_threshold: Option<i64>,
    /// Default image size of the user.
    pub default_image_size: Option<String>,
    /// Favorite tags the user has.
    pub favorite_tags: Option<String>,
    /// The user's blacklist tags.
    pub blacklisted_tags: Option<String>,
    /// The time zone of the user.
    pub time_zone: Option<String>,
    /// The post count per page.
    pub per_page: Option<i64>,
    /// Custom style/theme of E6.
    pub custom_style: Option<String>,
    /// Count of all the user's favorites.
    pub favorite_count: Option<i64>,
    /// The API regen multiplier.
    pub api_regen_multiplier: Option<i64>,
    /// The API burst limit.
    pub api_burst_limit: Option<i64>,
    /// The remaining API limit.
    pub remaining_api_limit: Option<i64>,
    /// The statement given while being in timeout.
    pub statement_timeout: Option<i64>,
    /// The limit for how many times a user can favorite.
    pub favorite_limit: Option<i64>,
    /// The maximum tag query limit, the amount amount of tags a user can search.
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

    /// If the client authenticated or not.
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

    /// Appends base url with id/name before ending with `.json`.
    pub fn append_url(&self, url: &str, append: &str) -> String {
        format!("{}{}.json", url, append)
    }

    /// Gets entry by type `T`, this is used for every request where the url needs to be appended to.
    pub fn get_entry_from_appended_id<T>(&self, id: &str, url_type_key: &str) -> T
        where
            T: DeserializeOwned,
    {
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

    /// Performs a bulk search for posts using tags to filter the response.
    pub fn bulk_search(&self, searching_tag: &str, page: u16) -> BulkPostEntry {
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
}

impl Clone for RequestSender {
    fn clone(&self) -> Self {
        RequestSender {
            client: self.client.clone(),
            urls: self.urls.clone(),
        }
    }
}
