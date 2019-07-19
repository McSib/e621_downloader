extern crate serde;

use serde::{Deserialize, Serialize};

/// If an error occurs from server, it will respond with this.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ErrorEntry {
    /// If the attempted grab is a success
    pub success: bool,
    /// Error message of failed grab if `success` is false
    pub msg: String,
}

/// Time the post was created.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeSet {
    pub json_class: String,
    /// Time in seconds
    pub s: i64,
    /// Time in nano-seconds
    pub n: i64,
}

/// GET return for set entry on e621/e926.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SetEntry {
    /// ID of set
    pub id: i64,
    /// Name of set
    pub name: String,
    /// Time the set was created
    pub created_at: TimeSet,
    /// Time the set was last updated
    pub updated_at: TimeSet,
    /// Id of user who created the set and updates it
    pub user_id: i64,
    /// Description of the set
    pub description: String,
    /// The short name of the set
    #[serde(rename = "shortname")]
    pub short_name: String,
    /// The amount of posts contained in the set
    pub post_count: i64,
    /// Ids for all posts in the set
    pub posts: Vec<i64>,
}

/// GET return for post entry on e621/e926.
///
/// # Important
///
/// `type_locked` can be null of not set by admins.
#[derive(Deserialize, Clone, Debug)]
pub struct TagEntry {
    /// Id of tag
    pub id: u32,
    /// Name of tag
    pub name: String,
    /// Number of all posts that use this tag
    pub count: u32,
    /// The type of tag it is.
    /// `0`: General; `1`: Artist; `2`: Nil (This used to be something, but was removed)
    #[serde(rename = "type")]
    pub tag_type: u8,
    /// If the type is locked (this value can also be [`None`])
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
    /// Time the pool was created
    pub created_at: TimeSet,
    /// Description of pool
    pub description: String,
    /// Id of pool
    pub id: i64,
    /// If the pool is active or not
    pub is_active: bool,
    /// If the pool is locked or not
    pub is_locked: bool,
    /// Name of pool
    pub name: String,
    /// The amount of posts added to the pool
    pub post_count: i64,
    /// Time the pool was updated
    pub updated_at: TimeSet,
    /// Id of user who created and updated post
    pub user_id: i64,
    /// All posts in pool
    pub posts: Vec<PostEntry>,
}
