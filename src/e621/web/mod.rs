extern crate reqwest;
extern crate serde;
extern crate pbr;

use serde::{Deserialize, Serialize};

use crate::e621::io::Config;
use std::error::Error;
use self::reqwest::Client;
use self::reqwest::header::USER_AGENT;
use crate::e621::io::tag::Tag;

static USER_AGENT_PROJECT_NAME: &'static str = "e621_downloader/0.0.1 (by McSib on e621";

/// Time the post was created.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatedAt {
    pub json_class: String,
    pub s: i64,
    pub n: i64,
}

/// Post from e621 or E926.
/// 
/// # Important
/// If the post that is loaded happens to be deleted when loaded, these properties will not be usable:
/// `source`, `sources`, `md5`, `file_size`, `file_ext`, `preview_width`, `preview_height`, `sample_url`, `sample_width`, `sample_height`, `has_children`, `children`.
#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    /// The ID of the post
    pub id: i64,
    /// Tags from the post
    pub  tags: String,
    /// Tags that are locked by the admins
    pub locked_tags: Option<String>,
    /// Description of the post
    pub description: String,
    /// When the post was uploaded
    pub created_at: CreatedAt,
    /// User ID of the user who uploaded the post
    pub creator_id: i64,
    /// Username of the user who uploaded the post
    pub author: String,
    /// The amount of changes that the post went through since uploaded
    pub change: i64,
    /// The main source of the work (use `sources` instead when using all source listed on post)
    pub source: Option<String>,
    /// How many upvoted or downvoted the post
    pub score: i64,
    /// How many favorites the post has
    pub fav_count: i64,
    /// The MD5 certification of the post
    pub md5: Option<String>,
    /// Size of the source file
    pub file_size: Option<i64>,
    /// URL of the source file
    pub file_url: String,
    /// Extension of the source file (png, jpg, webm, gif, etc)
    pub file_ext: Option<String>,
    /// URL for the preview file
    pub preview_url: String,
    /// Width of the preview file
    pub preview_width: Option<i64>,
    /// Height of the preview file
    pub preview_height: Option<i64>,
    /// URL for the sample file
    pub sample_url: Option<String>,
    /// Width of the sample file
    pub sample_width: Option<i64>,
    /// Height of the sample file
    pub sample_height: Option<i64>,
    /// Rating of the post (safe, questionable, explicit), this will be "s", "q", "e"
    pub rating: String,
    /// Post status, one of: active, flagged, pending, deleted
    pub status: String,
    /// Width of image
    pub width: i64,
    /// Height of image
    pub height: i64,
    /// If the post has comments
    pub has_comments: bool,
    /// If the post has notes
    pub has_notes: bool,
    /// If the post has children
    pub has_children: Option<bool>,
    /// All of the children attached to post
    pub children: Option<String>,
    /// If this post is a child, this will be the parent post's ID
    pub parent_id: Option<i64>,
    /// The artist or artists that drew this image
    pub artist: Vec<String>,
    /// All the sources for the work
    pub sources: Option<Vec<String>>,
}

/// Basic web connector for e621.
pub struct EWeb {
    /// Url used for connecting and downloading images
    pub url: String,
    /// Whether the site is the safe version or note. If true, it will force connection to E926 instead of E621
    safe: bool,
    /// Configuration data used for downloading images and tag searches
    config: Config,
    /// Web client to connect and download images.
    client: Client,
}

impl EWeb {
    /// Creates new EWeb object for connecting and downloading images.
    ///
    /// # Example
    ///
    /// ```
    /// let connector = EWeb::new();
    /// ```
    pub fn new(config: &Config) -> EWeb {
        EWeb {
            url: "https://e621.net/post/index.json".to_string(),
            safe: false,
            config: config.clone(),
            client: Client::new(),
        }
    }

    /// Sets the site into safe mode so no NSFW images popup in the course of downloading.
    ///
    /// # Example
    ///
    /// ```
    /// let connector = EWeb::new();
    /// connector.set_safe();
    /// ```
    pub fn set_safe(&mut self) {
        self.safe = true;
        self.update_to_safe_url();
    }

    /// Gets posts with tags supplied and iterates through pages until no more posts available.
    pub fn get_posts(&mut self, tags: &Vec<Tag>) -> Result<Vec<Post>, Box<Error>> {
        let mut page = 1;
        let mut posts: Vec<Post> = Vec::new();
        let mut json: Vec<Post>;

        let mut tag_string = String::new();
        for tag in tags {
            tag_string.push_str(format!("{} ", tag.value).as_str());
        }

        loop {
            json = self.client.get(&self.url)
                            .header(USER_AGENT, USER_AGENT_PROJECT_NAME)
                .query(&[("tags", format!("{}date:>={}", tag_string, self.config.last_run)),
                    ("page", format!("{}", page)),
                    ("limit", String::from("1000"))])
                .send()?
                .json::<Vec<Post>>()?;
            if json.len() <= 0 {
                break;
            }

            posts.append(&mut json);
            page += 1;
        }

        Ok(posts)
    }

    /// Updates the url for safe mode.
    fn update_to_safe_url(&mut self) {
        self.url = self.url.replace("e621", "e926");
    }
}
