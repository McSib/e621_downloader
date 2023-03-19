/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */


use std::any::type_name;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{AUTHORIZATION, USER_AGENT};
use serde::de::DeserializeOwned;
use serde_json::{from_value, Value};

use crate::e621::io::{emergency_exit, Login};
use crate::e621::sender::entries::{AliasEntry, BulkPostEntry, PostEntry, TagEntry};

pub(crate) mod entries;

/// Creates a hashmap through similar syntax of the `vec` macro.
///
/// # Arguments
/// * `x`: Represents multiple tuples being passed as parameter.
///
/// # Example
///
/// ```rust
/// # use std::collections::hashmap;
///
/// let hashmap = hashmap![("Testing", "testing"), ("Example", "example")];
///
/// assert_eq!(hashmap["Testing"], String::from("testing"));
/// ```
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

/// Default user agent value.
const USER_AGENT_VALUE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    "/",
    env!("CARGO_PKG_VERSION"),
    " (by ",
    env!("CARGO_PKG_AUTHORS"),
    " on e621)"
);

/// A reference counted client used for all searches by the [Grabber], [Blacklist], [E621WebConnector], etc.
struct SenderClient {
    /// [Client] wrapped in a [Rc] so only one instance of the client exists. This will prevent an overabundance of
    /// clients in the code.
    client: Rc<Client>,
    /// The base64 encrypted username and password of the user. This is passed only through the [AUTHORIZATION] header
    /// of the request and is a highly secured method of login through client.
    auth: Rc<String>,
}

impl SenderClient {
    /// Creates root client.
    fn new(auth: String) -> Self {
        trace!("SenderClient initializing with USER_AGENT_VALUE \"{USER_AGENT_VALUE}\"");

        SenderClient {
            client: Rc::new(SenderClient::build_client()),
            auth: Rc::new(auth),
        }
    }

    /// Runs client through a builder to give it required settings.
    /// Cookies aren't stored in the client, TCP_NODELAY is on, and timeout is changed from 30 seconds to 60.
    fn build_client() -> Client {
        Client::builder()
            .use_rustls_tls()
            .http2_prior_knowledge()
            .tcp_keepalive(Duration::from_secs(30))
            .tcp_nodelay(true)
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new())
    }

    /// A wrapping function that acts the exact same as `self.client.get` but will instead attach the user agent header
    /// before returning the [RequestBuilder]. This will ensure that all requests sent have the proper user agent info.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to request.
    ///
    /// returns: RequestBuilder
    pub(crate) fn get(&self, url: &str) -> RequestBuilder {
        self.client.get(url).header(USER_AGENT, USER_AGENT_VALUE)
    }

    /// This is the same as `self.get(url)` but will attach the authorization header with username and API hash.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to request.
    ///
    /// returns: RequestBuilder
    pub(crate) fn get_with_auth(&self, url: &str) -> RequestBuilder {
        if self.auth.is_empty() {
            self.get(url)
        } else {
            self.get(url).header(AUTHORIZATION, self.auth.as_str())
        }
    }
}

impl Clone for SenderClient {
    /// Creates a new instance of SenderClient, but clones the [Rc] of the root client, ensuring that all requests are
    /// going to the same client.
    fn clone(&self) -> Self {
        SenderClient {
            client: Rc::clone(&self.client),
            auth: Rc::clone(&self.auth),
        }
    }
}

/// A sender that handles direct calls to the API.
///
/// This acts as a safety layer to ensure calls to the API are less error prone.
pub(crate) struct RequestSender {
    /// The client that will be used to send all requests.
    ///
    /// Even though the [SenderClient] isn't wrapped in a [Rc], the main client inside of it is, this will ensure that
    /// all request are only sent through one client.
    client: SenderClient,
    /// All available urls to use with the sender.
    urls: Rc<RefCell<HashMap<String, String>>>,
}

impl RequestSender {
    pub(crate) fn new() -> Self {
        let login = Login::get();
        let auth = if login.is_empty() {
            String::new()
        } else {
            base64_url::encode(format!("{}:{}", login.username(), login.api_key()).as_str())
        };

        RequestSender {
            client: SenderClient::new(auth),
            urls: Rc::new(RefCell::new(RequestSender::initialize_url_map())),
        }
    }

    /// Initializes all the urls that will be used by the sender.
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
    pub(crate) fn is_authenticated(&self) -> bool {
        !self.client.auth.is_empty()
    }

    /// Updates all the urls from e621 to e926.
    pub(crate) fn update_to_safe(&mut self) {
        self.urls
            .borrow_mut()
            .iter_mut()
            .for_each(|(_, value)| *value = value.replace("e621", "e926"));
    }

    /// If a request failed, this will output what type of error it is before exiting.
    ///
    /// # Arguments
    ///
    /// * `error`: The type of error thrown.
    fn output_error(&self, error: &reqwest::Error) {
        error!(
            "Error occurred from sent request. \
             Error: {error}",
        );
        trace!("Url where error occurred: {:#?}", error.url());

        if let Some(status) = error.status() {
            let code = status.as_u16();
            trace!("The response code from the server was: {code}");

            const SERVER_INTERNAL: u16 = 500;
            const SERVER_RATE_LIMIT: u16 = 503;
            const CLIENT_FORBIDDEN: u16 = 403;
            const CLIENT_THROTTLED: u16 = 421;
            match code {
                SERVER_INTERNAL => {
                    error!(
                        "There was an error that happened internally in the servers, \
                         please try using the downloader later until the issue is solved."
                    );
                }
                SERVER_RATE_LIMIT => {
                    error!(
                        "Server could not handle the request, or the downloader has \
                         exceeded the rate-limit. Contact the developer immediately about this \
                         issue."
                    );
                }
                CLIENT_FORBIDDEN => {
                    error!(
                        "The client was forbidden from accessing the api, contact the \
                         developer immediately if this error occurs."
                    );
                }
                CLIENT_THROTTLED => {
                    error!(
                        "The user is throttled, thus the request is unsuccessful. \
                         Contact the developer immediately if this error occurs."
                    );
                }
                _ => {
                    error!("Response code couldn't be posted...");
                }
            }
        }

        emergency_exit("To prevent the program from crashing, it will do an emergency exit.");
    }

    /// Gets the response from a sent request and checks to ensure it was successful.
    ///
    /// # Arguments
    ///
    /// * `result`: The result to check.
    ///
    /// returns: Response
    pub(crate) fn check_response(&self, result: Result<Response, reqwest::Error>) -> Response {
        match result {
            Ok(response) => response,
            Err(ref error) => {
                self.output_error(error);
                unreachable!()
            }
        }
    }

    /// Sends request to download image.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to the file to download.
    /// * `file_size`: The file size of the file.
    ///
    /// returns: Vec<u8, Global>
    pub(crate) fn download_image(&self, url: &str, file_size: i64) -> Vec<u8> {
        let mut image_response = self.check_response(self.client.get(url).send());
        let mut image_bytes: Vec<u8> = Vec::with_capacity(file_size as usize);
        image_response
            .copy_to(&mut image_bytes)
            .with_context(|| {
                "Failed to download image!".to_string()
            })
            .unwrap();

        image_bytes
    }

    /// Appends base url with id/name before ending with `.json`.
    ///
    /// # Arguments
    ///
    /// * `url`: The url to change.
    /// * `append`: The id/name.
    ///
    /// returns: String
    pub(crate) fn append_url(&self, url: &str, append: &str) -> String {
        format!("{url}{append}.json")
    }

    /// Gets entry by type `T`, this is used for every request where the url needs to be appended to.
    ///
    /// # Arguments
    ///
    /// * `id`: The id to search for.
    /// * `url_type_key`: The type of url to use.
    ///
    /// returns: T
    pub(crate) fn get_entry_from_appended_id<T>(&self, id: &str, url_type_key: &str) -> T
    where
        T: DeserializeOwned,
    {
        let value: Value = self
            .check_response(
                self.client
                    .get_with_auth(&self.append_url(&self.urls.borrow()[url_type_key], id))
                    .send(),
            )
            .json()
            .with_context(|| {
                format!(
                    "Json was unable to deserialize to \"{}\"!\n\
                     url_type_key: {}\n\
                     id: {}",
                    type_name::<Value>(), url_type_key, id
                )
            })
            .unwrap();
        match url_type_key {
            "single" => from_value(value.get("post").unwrap().clone())
                .with_context(|| {
                    error!(
                        "Could not convert single post to type \"{}\"!",
                        type_name::<T>()
                    );
                    "Unexpected error occurred when trying to perform conversion from value to entry type above.".to_string()
                })
                .unwrap(),
            _ => from_value(value)
                .with_context(|| {
                    error!(
                        "Could not convert entry to type \"{}\"!",
                        type_name::<T>()
                    );
                    "Unexpected error occurred when trying to perform conversion from value to entry type above.".to_string()
                })
                .unwrap(),
        }
    }

    /// Performs a bulk search for posts using tags to filter the response.
    ///
    /// # Arguments
    ///
    /// * `searching_tag`: The tags for filtering.
    /// * `page`: The page to search for.
    ///
    /// returns: BulkPostEntry
    pub(crate) fn bulk_search(&self, searching_tag: &str, page: u16) -> BulkPostEntry {
        debug!("Downloading page {page} of tag {searching_tag}");

        self.check_response(
            self.client
                .get_with_auth(&self.urls.borrow()["posts"])
                .query(&[
                    ("tags", searching_tag),
                    ("page", &format!("{page}")),
                    ("limit", &320.to_string()),
                ])
                .send(),
        )
        .json()
        .with_context(|| {
            error!(
                "Unable to deserialize json to \"{}\"!",
                type_name::<Vec<PostEntry>>()
            );
            "Failed to perform bulk search...".to_string()
        })
        .unwrap()
    }

    /// Gets tags by their name.
    ///
    /// # Arguments
    ///
    /// * `tag`: The name of the tag.
    ///
    /// returns: Vec<TagEntry, Global>
    pub(crate) fn get_tags_by_name(&self, tag: &str) -> Vec<TagEntry> {
        let result: Value = self
            .check_response(
                self.client
                    .get(&self.urls.borrow()["tag_bulk"])
                    .query(&[("search[name]", tag)])
                    .send(),
            )
            .json()
            .with_context(|| {
                format!(
                    "Json was unable to deserialize to \"{}\"!\n\
                     url_type_key: tag_bulk\n\
                     tag: {}",
                    type_name::<Value>(), tag
                )
            })
            .unwrap();
        if result.is_object() {
            vec![]
        } else {
            from_value::<Vec<TagEntry>>(result)
                .with_context(|| {
                    error!(
                        "Unable to deserialize Value to \"{}\"!",
                        type_name::<Vec<TagEntry>>()
                    );
                    "Failed to perform bulk search...".to_string()
                })
                .unwrap()
        }
    }

    /// Queries aliases and returns response.
    ///
    /// # Arguments
    ///
    /// * `tag`: The alias to search for.
    ///
    /// returns: Option<Vec<AliasEntry, Global>>
    ///
    /// # Examples
    ///
    /// ```
    ///
    /// ```
    pub(crate) fn query_aliases(&self, tag: &str) -> Option<Vec<AliasEntry>> {
        let result = self
            .check_response(
                self.client
                    .get(&self.urls.borrow()["alias"])
                    .query(&[
                        ("commit", "Search"),
                        ("search[name_matches]", tag),
                        ("search[order]", "status"),
                    ])
                    .send(),
            )
            .json::<Vec<AliasEntry>>();

        match result {
            Ok(e) => Some(e),
            Err(e) => {
                trace!("No alias was found for {tag}...");
                trace!("Printing trace message for why None was returned...");
                trace!("{}", e.to_string());
                None
            }
        }
    }
}

impl Clone for RequestSender {
    fn clone(&self) -> Self {
        RequestSender {
            client: self.client.clone(),
            urls: Rc::clone(&self.urls),
        }
    }
}
