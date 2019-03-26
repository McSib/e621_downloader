extern crate reqwest;
extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};

use crate::e621::io::Config;

/// Time the post was created.
#[derive(Serialize, Deserialize)]
pub struct CreatedAt {
    json_class: String,
    s: i64,
    n: i64,
}

/// Post from e621 or E926.
/// 
/// # Errors
/// If the post that is loaded happens to be deleted when loaded, it will crash the program as
/// `source`, `sources`, `md5`, `file_size`, `file_ext`, `preview_width`, `preview_height`, `sample_url`, `sample_width`, `sample_height`, `has_children`, `children`
/// will be null.
#[derive(Serialize, Deserialize)]
pub struct Post {
    /// The ID of the post
    id: i64,
    /// Tags from the post
    tags: String,
    /// Tags that are locked by the admins
    locked_tags: String,
    /// Description of the post
    description: String,
    /// When the post was uploaded
    created_at: CreatedAt,
    /// User ID of the user who uploaded the post
    creator_id: i64,
    /// Username of the user who uploaded the post
    author: String,
    /// The amount of changes that the post went through since uploaded
    change: i64,
    /// The main source of the work (use `sources` instead when using all source listed on post)
    source: String,
    /// How many upvoted or downvoted the post
    score: i64,
    /// How many favorites the post has
    fav_count: i64,
    /// The MD5 certification of the post
    md5: String,
    /// Size of the source file
    file_size: i64,
    /// URL of the source file
    file_url: String,
    /// Extension of the source file (png, jpg, webm, gif, etc)
    file_ext: String,
    /// URL for the preview file
    preview_url: String,
    /// Width of the preview file
    preview_width: i64,
    /// Height of the preview file
    preview_height: i64,
    /// URL for the sample file
    sample_url: String,
    /// Width of the sample file
    sample_width: i64,
    /// Height of the sample file
    sample_height: i64,
    /// Rating of the post (safe, questionable, explicit), this will be "s", "q", "e"
    rating: String,
    /// Post status, one of: active, flagged, pending, deleted
    status: String,
    /// Width of image
    width: i64,
    /// Height of image
    height: i64,
    /// If the post has comments
    has_comments: bool,
    /// If the post has notes
    has_notes: bool,
    /// If the post has children
    has_children: bool,
    /// All of the children attached to post
    children: String,
    /// If this post is a child, this will be the parent post's ID
    parent_id: String,
    /// The artist or artists that drew this image
    artist: Vec<String>,
    /// All the sources for the work
    sources: Vec<String>,
}

/// Basic web connector for e621.
pub struct EWeb {
    /// Url used for connecting and downloading images
    pub url: String,
    /// Whether the site is the safe version or note. If true, it will force connection to E926 instead of E621
    safe: bool,
    /// Configuration data used for downloading images and tag searches
    config: Config,
}

impl EWeb {
    /// Creates new EWeb object for connecting and downloading images.
    /// 
    /// ```
    /// let connector = EWeb::new();
    /// ```
    pub fn new(config: &Config) -> EWeb {
        EWeb {
            url: "https://e621.net/post/index.json".to_string(),
            safe: false,
            config: config.clone(),
        }
    }

    /// Sets the site into safe mode so no NSFW images popup in the course of downloading.
    /// 
    /// ```
    /// let connector = EWeb::new();
    /// connector.set_safe();
    /// ```
    pub fn set_safe(&mut self) {
        self.safe = true;
        self.update_to_safe_url();
    }

    /// Updates the url for safe mode.
    fn update_to_safe_url(&mut self) {
        self.url = self.url.replace("e621", "e926");
    }

    /// Tests the connections before doing further work with the site.
    fn test_connection(&self) -> Result<(), Box<std::error::Error>> {
        reqwest::get(self.url.as_str())?;
        Ok(())
    }
}
