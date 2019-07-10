extern crate serde;

use serde::{Deserialize, Serialize};

/// If an error occurs from server, it will respond with this.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorEntry {
    pub success: bool,
    pub msg: String,
}

/// Time the post was created.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeSet {
    pub json_class: String,
    pub s: i64,
    pub n: i64,
}

/// GET return for set entry on e621/e926.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetEntry {
    pub id: i64,
    pub name: String,
    pub created_at: TimeSet,
    pub updated_at: TimeSet,
    pub user_id: i64,
    pub description: String,
    #[serde(rename = "shortname")]
    pub short_name: String,
    pub post_count: i64,
    pub posts: Vec<i64>,
}

/// GET return for post entry on e621/e926.
///
/// # Important
/// `type_locked` can be null of not set by admins.
#[derive(Deserialize, Clone, Debug)]
pub struct TagEntry {
    pub id: u32,
    pub name: String,
    pub count: u32,
    #[serde(rename = "type")]
    pub tag_type: u8,
    pub type_locked: Option<bool>,
}

/// GET return for post entry on e621/e926.
///
/// # Important
/// If the post that is loaded happens to be deleted when loaded, these properties will not be usable:
/// `source`, `sources`, `md5`, `file_size`, `file_ext`, `preview_width`, `preview_height`, `sample_url`, `sample_width`, `sample_height`, `has_children`, `children`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostEntry {
    /// The ID of the post
    pub id: i64,
    /// Tags from the post
    pub tags: String,
    /// Tags that are locked by the admins
    pub locked_tags: Option<String>,
    /// Description of the post
    pub description: String,
    /// When the post was uploaded
    pub created_at: TimeSet,
    /// User ID of the user who uploaded the post
    pub creator_id: Option<i64>,
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

/// GET return for pool entry on e621/e926.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PoolEntry {
    pub created_at: TimeSet,
    pub description: String,
    pub id: i64,
    pub is_active: bool,
    pub is_locked: bool,
    pub name: String,
    pub post_count: i64,
    pub updated_at: TimeSet,
    pub user_id: i64,
    pub posts: Vec<PostEntry>,
}
