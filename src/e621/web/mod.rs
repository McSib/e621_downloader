extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate pbr;

use serde::{Deserialize, Serialize};

use crate::e621::io::Config;
use std::error::Error;
use core::borrow::BorrowMut;
use self::reqwest::Url;
use self::pbr::ProgressBar;

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


    /// Adds array of tags into url.
    ///
    /// # Example
    ///
    /// ```
    /// let tags: Vec<&str> = vec!(["Hello", "What", "Yay"]);
    /// let connector = EWeb::new();
    /// connector.add_tags(&tags);
    /// ```
    pub fn add_tags(&mut self, tags: &Vec<&str>) {
        if tags.len() > 0 {
            let mut url_tags = String::new();
            for i in 0..tags.len() {
                if i != tags.len() - 1 {
                    url_tags.push_str(format!("{} ", tags[i]).as_str());
                } else {
                    url_tags.push_str(tags[i]);
                }
            }

            self.url.push_str(format!("?tags={} date:>={}&page=1", url_tags, self.config.last_run).as_str());
        }
    }

    /// Gets posts with tags supplied and iterates through pages until no more posts available.
    pub fn get_posts(&mut self) -> Result<Vec<Post>, Box<Error>> {
        let mut count = 0;
        self.get_count(&mut count)?;

        let mut page = 1;
        let mut posts: Vec<Post> = Vec::new();
        let mut json: Vec<Post> = reqwest::get(Url::parse(&self.url)?.as_str())?.json()?;
        let mut progress_bar = ProgressBar::new(count as u64);
        while json.len() > 0 {
            posts.append(&mut json);
            progress_bar.set(posts.len() as u64);
            page += 1;
            self.update_url_page(&page);
            json = reqwest::get(Url::parse(&self.url)?.as_str())?.json()?;
        }

        progress_bar.finish_println("Posts indexed...");

        Ok(posts)
    }

    /// TODO: Remove this function when it is no longer needed!
    ///
    /// Gets the count of all posts about to be grabbed.
    fn get_count(&mut self, count: &mut usize) -> Result<(), Box<Error>> {
        let mut page = 1;
        let mut count_finder: Vec<Post> = reqwest::get(Url::parse(&self.url)?.as_str())?.json()?;
        while count_finder.len() > 0 {
            *count += count_finder.len();
            page += 1;
            self.update_url_page(&page);
            count_finder = reqwest::get(Url::parse(&self.url)?.as_str())?.json()?;
        }

        Ok(())
    }

    /// TODO: Find a better method to replace this function!
    ///
    /// Updates the url with new page to be used.
    fn update_url_page(&mut self, new_page: &i32) {
        self.url = self.url.trim_end_matches(char::is_numeric).to_string();
        self.url.push_str(format!("{}", new_page).as_str());
    }

    /// Updates the url for safe mode.
    fn update_to_safe_url(&mut self) {
        self.url = self.url.replace("e621", "e926");
    }
}
