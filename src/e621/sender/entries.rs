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

use serde::{Deserialize, Serialize};

use crate::e621::io::tag::TagType;

/// GET return of alias entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AliasEntry {
    /// Alias ID.
    pub(crate) id: i64,
    /// Alias name.
    pub(crate) antecedent_name: String,
    /// Reason for the alias.
    pub(crate) reason: String,
    /// ID of the creator of the alias.
    pub(crate) creator_id: i64,
    /// The date the alias was created.
    pub(crate) created_at: Option<String>,
    /// Forum post id tied to the request for the alias to be approved.
    pub(crate) forum_post_id: Option<i64>,
    /// The date for when the alias was updated.
    pub(crate) updated_at: Option<String>,
    /// Forum topic ID for the thread where the request for alias approval was created.
    pub(crate) forum_topic_id: Option<i64>,
    /// Original tag name.
    pub(crate) consequent_name: String,
    /// Current status of the alias.
    /// Can be `approved`, `active`, `pending`, `deleted`, `retired`, `processing`, and `queued`.
    ///
    /// # Error
    /// Optionally, there can also be an `error` prompt with the following format:
    /// `"error: cannot update a new record"`
    /// ## Reason for Error
    /// This is probably an internal error with the server, and while it is exceptionally rare,
    /// there is still a probability.
    pub(crate) status: String,
    /// The amount of post the aliased tag is tied to.
    pub(crate) post_count: i64,
    /// ID of the user that approved the alias.
    pub(crate) approver_id: Option<i64>,
}

/// GET return of tag entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TagEntry {
    /// Id of the tag.
    pub(crate) id: i64,
    /// Name of the tag.
    pub(crate) name: String,
    /// Amount of posts that uses the tag.
    pub(crate) post_count: i64,
    /// Related tags that this tag is commonly paired with.
    pub(crate) related_tags: String,
    /// Most recent date the `related_tags` was updated.
    pub(crate) related_tags_updated_at: String,
    /// The type of tag it is.
    ///
    /// This tag can be the following types:
    /// - `0`: General;
    /// - `1`: Artist;
    /// - `2`: Nil (This used to be something, but was removed);
    /// - `3`: Copyright;
    /// - `4`: Character;
    /// - `5`: Species;
    pub(crate) category: u8,
    /// Whether or not the tag is locked.
    pub(crate) is_locked: bool,
    /// The date the tag was created.
    pub(crate) created_at: String,
    /// The date the tag was updated.
    pub(crate) updated_at: String,
}

impl TagEntry {
    /// Constrains the `TagType` enum to a tags type specifically.
    ///
    /// This can only be `TagType::General` or `TagType::Artist`.
    pub(crate) fn to_tag_type(&self) -> TagType {
        match self.category {
            // `0`: General; `3`: Copyright; `5`: Species; `4`: Character; `6`: Invalid;
            // `7`: Meta; `8`: Lore;
            0 | 3..=8 => TagType::General,
            // `1`: Artist;
            1 => TagType::Artist,
            _ => unreachable!(),
        }
    }
}

/// Wrapper struct that holds the return of bulk searches.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct BulkPostEntry {
    /// All posts in the bulk.
    pub(crate) posts: Vec<PostEntry>,
}

/// GET return of post entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PostEntry {
    /// The ID number of the post.
    pub(crate) id: i64,
    /// The time the post was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    ///  The time the post was last updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: Option<String>,
    /// The main image of the post.
    pub(crate) file: File,
    /// The preview image of the post.
    pub(crate) preview: Preview,
    /// The sample image of the post.
    pub(crate) sample: Sample,
    /// The score of the post.
    pub(crate) score: Score,
    /// The tags tied to the post.
    pub(crate) tags: Tags,
    /// An array of tags that are locked on the post.
    pub(crate) locked_tags: Vec<String>,
    /// An ID that increases for every post alteration on E6 (explained below)
    ///
    /// `change_seq` is a number that is increased every time a post is changed on the site.
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
    pub(crate) change_seq: i64,
    /// All the flags that could be raised on the post.
    pub(crate) flags: Flags,
    /// The post’s rating. Either `s`, `q` or `e`.
    pub(crate) rating: String,
    /// How many people have favorited the post.
    pub(crate) fav_count: i64,
    /// The source field of the post.
    pub(crate) sources: Vec<String>,
    /// An array of Pool IDs that the post is a part of.
    pub(crate) pools: Vec<i64>,
    /// The relationships of the post.
    pub(crate) relationships: Relationships,
    /// The ID of the user that approved the post, if available.
    pub(crate) approver_id: Option<i64>,
    /// The ID of the user that uploaded the post.
    pub(crate) uploader_id: i64,
    /// The post’s description.
    pub(crate) description: String,
    /// The count of comments on the post.
    pub(crate) comment_count: i64,
    /// If provided auth credentials, will return if the authenticated user has favorited the post or not.
    /// HTTP Basic Auth is recommended over `login` and `api_key` parameters in the URL.
    pub(crate) is_favorited: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct File {
    /// The width of the post.
    pub(crate) width: i64,
    /// The height of the post.
    pub(crate) height: i64,
    /// The file’s extension.
    pub(crate) ext: String,
    /// The size of the file in bytes.
    pub(crate) size: i64,
    /// The md5 of the file.
    pub(crate) md5: String,
    /// The URL where the file is hosted on E6
    pub(crate) url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Preview {
    /// The width of the post preview.
    pub(crate) width: i64,
    /// The height of the post preview.
    pub(crate) height: i64,
    /// The URL where the preview file is hosted on E6
    pub(crate) url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Sample {
    ///  If the post has a sample/thumbnail or not.
    pub(crate) has: Option<bool>,
    /// The width of the post sample.
    pub(crate) height: i64,
    /// The height of the post sample.
    pub(crate) width: i64,
    /// The URL where the sample file is hosted on E6.
    pub(crate) url: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Score {
    /// The number of times voted up.
    pub(crate) up: i64,
    /// A negative number representing the number of times voted down.
    pub(crate) down: i64,
    /// The total score (up + down).
    pub(crate) total: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Tags {
    /// An array of all the `general` tags on the post.
    pub(crate) general: Vec<String>,
    /// An array of all the `species` tags on the post.
    pub(crate) species: Vec<String>,
    /// An array of all the `character` tags on the post.
    pub(crate) character: Vec<String>,
    /// An array of all the `copyright` tags on the post.
    pub(crate) copyright: Vec<String>,
    /// An array of all the `artist` tags on the post.
    pub(crate) artist: Vec<String>,
    /// An array of all the `invalid` tags on the post.
    pub(crate) invalid: Vec<String>,
    /// An array of all the `lore` tags on the post.
    pub(crate) lore: Vec<String>,
    /// An array of all the `meta` tags on the post.
    pub(crate) meta: Vec<String>,
}

impl Tags {
    /// Consumes and combines all of the tags into a single array.
    pub(crate) fn combine_tags(self) -> Vec<String> {
        vec![
            self.general,
            self.species,
            self.character,
            self.copyright,
            self.artist,
            self.invalid,
            self.lore,
            self.meta,
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Flags {
    /// If the post is pending approval.
    pub(crate) pending: bool,
    /// If the post is flagged for deletion.
    pub(crate) flagged: bool,
    /// If the post has it’s notes locked.
    pub(crate) note_locked: bool,
    /// If the post’s status has been locked.
    pub(crate) status_locked: Option<bool>,
    /// If the post’s rating has been locked.
    pub(crate) rating_locked: bool,
    /// If the post has been deleted.
    pub(crate) deleted: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Relationships {
    /// The ID of the post’s parent, if it has one.
    pub(crate) parent_id: Option<i64>,
    /// If the post has child posts.
    pub(crate) has_children: bool,
    pub(crate) has_active_children: bool,
    /// A list of child post IDs that are linked to the post, if it has any.
    pub(crate) children: Vec<i64>,
}

/// GET return of set entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SetEntry {
    /// The ID of the set.
    pub(crate) id: i64,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: String,
    /// The ID of the user that created the set.
    pub(crate) creator_id: i64,
    /// If the set is pub(crate) ic and visible.
    pub(crate) is_pubic: bool,
    /// The name of the set.
    pub(crate) name: String,
    /// The short name of the set.
    pub(crate) shortname: String,
    /// The description of the set.
    pub(crate) description: String,
    /// The amount of posts in the set.
    pub(crate) post_count: i64,
    /// If the set will transfer its post on delete.
    pub(crate) transfer_on_delete: bool,
    /// An array group of posts in the pool.
    pub(crate) post_ids: Vec<i64>,
}

/// GET return of pool entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PoolEntry {
    /// The ID of the pool.
    pub(crate) id: i64,
    /// The name of the pool.
    pub(crate) name: String,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: String,
    /// The ID of the user that created the pool.
    pub(crate) creator_id: i64,
    /// The description of the pool.
    pub(crate) description: String,
    /// If the pool is active and still getting posts added.
    pub(crate) is_active: bool,
    /// Can be `series` or `collection`.
    pub(crate) category: String,
    /// An array group of posts in the pool.
    pub(crate) post_ids: Vec<i64>,
    /// The name of the user that created the pool.
    pub(crate) creator_name: String,
    /// The amount of posts in the pool.
    pub(crate) post_count: i64,
}

/// GET return of user entry for e621/e926.
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UserEntry {
    /// The amount of wiki changes made by the user.
    pub(crate) wiki_page_version_count: i64,
    /// The amount of artist changes made by the user.
    pub(crate) artist_version_count: i64,
    /// The amount of pool changes made by the user.
    pub(crate) pool_version_count: i64,
    /// The amount of post changes made by the user.
    pub(crate) forum_post_count: i64,
    /// Count of comments posted by the user.
    pub(crate) comment_count: i64,
    /// Count of flags done by the user.
    pub(crate) flag_count: i64,
    /// The amount of positive feedback given by the user.
    pub(crate) positive_feedback_count: i64,
    /// The amount of neutral feedback given by the user.
    pub(crate) neutral_feedback_count: i64,
    /// The amount of negative feedback given by the user.
    pub(crate) negative_feedback_count: i64,
    /// Upload limit of the user.
    pub(crate) upload_limit: i64,
    /// ID of the user.
    pub(crate) id: i64,
    /// The time the pool was created in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) created_at: String,
    /// Name of the user.
    pub(crate) name: String,
    /// Level of the user.
    pub(crate) level: i64,
    /// Base upload limit of the user.
    pub(crate) base_upload_limit: i64,
    /// Count of posts uploaded by the user.
    pub(crate) post_upload_count: i64,
    /// Count of posts updated by the user.
    pub(crate) post_update_count: i64,
    /// Count of notes updated by the user.
    pub(crate) note_update_count: i64,
    /// If user is banned or not.
    pub(crate) is_banned: bool,
    /// Whether or not the user can approve posts.
    pub(crate) can_approve_posts: bool,
    /// Whether or not uploading posts affect the post limit.
    pub(crate) can_upload_free: bool,
    /// The string of the user's current level.
    pub(crate) level_string: String,
    /// Whether or not avatars should be shown.
    pub(crate) show_avatars: Option<bool>,
    /// Whether or not the blacklist should block avatars.
    pub(crate) blacklist_avatars: Option<bool>,
    /// Whether or not the blacklist should block users.
    pub(crate) blacklist_users: Option<bool>,
    /// Whether or not a post's description should be collapsed initially.
    pub(crate) description_collapsed_initially: Option<bool>,
    /// Whether or not comments should be hidden.
    pub(crate) hide_comments: Option<bool>,
    /// Whether or not hidden comments should be shown.
    pub(crate) show_hidden_comments: Option<bool>,
    /// Whether or not to show post statistics.
    pub(crate) show_post_statistics: Option<bool>,
    /// Whether or not the user has mail.
    pub(crate) has_mail: Option<bool>,
    /// Whether or not the user will receive email notifications.
    pub(crate) receive_email_notifications: Option<bool>,
    /// Whether or not keyboard navigation is on/off.
    pub(crate) enable_keyboard_navigation: Option<bool>,
    /// Whether or not privacy mode is enabled.
    pub(crate) enable_privacy_mode: Option<bool>,
    /// Whether or not usernames should be styled.
    pub(crate) style_usernames: Option<bool>,
    /// Whether auto complete should be on or off.
    pub(crate) enable_auto_complete: Option<bool>,
    /// Whether or not searches should be saved.
    pub(crate) has_saved_searches: Option<bool>,
    /// Whether or not thumbnails should be cropped.  
    pub(crate) disable_cropped_thumbnails: Option<bool>,
    /// Whether or not mobile gestures should be on or off.
    pub(crate) disable_mobile_gestures: Option<bool>,
    /// Whether or not safe mode is on/off.
    pub(crate) enable_safe_mode: Option<bool>,
    /// Whether or not responsive mode is disabled.
    pub(crate) disable_responsive_mode: Option<bool>,
    /// Whether or not post tooltips is disabled.
    pub(crate) disable_post_tooltips: Option<bool>,
    /// Whether or not the user can't flag.
    pub(crate) no_flagging: Option<bool>,
    /// Whether or not the user can't give feedback.
    pub(crate) no_feedback: Option<bool>,
    /// Whether or not dmail is disabled.
    pub(crate) disable_user_dmails: Option<bool>,
    /// Whether or not compact uploader is enabled.
    pub(crate) enable_compact_uploader: Option<bool>,
    /// The time the pool was updated in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) updated_at: Option<String>,
    /// The user's email.
    pub(crate) email: Option<String>,
    /// The time the user was last logged in in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) last_logged_in_at: Option<String>,
    /// The time the last forum the user read in the format of `YYYY-MM-DDTHH:MM:SS.MS+00:00`.
    pub(crate) last_forum_read_at: Option<String>,
    /// Recent tags searched by the user.
    pub(crate) recent_tags: Option<String>,
    /// Comment threshold of the user.
    pub(crate) comment_threshold: Option<i64>,
    /// Default image size of the user.
    pub(crate) default_image_size: Option<String>,
    /// Favorite tags the user has.
    pub(crate) favorite_tags: Option<String>,
    /// The user's blacklist tags.
    pub(crate) blacklisted_tags: Option<String>,
    /// The time zone of the user.
    pub(crate) time_zone: Option<String>,
    /// The post count per page.
    pub(crate) per_page: Option<i64>,
    /// Custom style/theme of E6.
    pub(crate) custom_style: Option<String>,
    /// Count of all the user's favorites.
    pub(crate) favorite_count: Option<i64>,
    /// The API regen multiplier.
    pub(crate) api_regen_multiplier: Option<i64>,
    /// The API burst limit.
    pub(crate) api_burst_limit: Option<i64>,
    /// The remaining API limit.
    pub(crate) remaining_api_limit: Option<i64>,
    /// The statement given while being in timeout.
    pub(crate) statement_timeout: Option<i64>,
    /// The limit for how many times a user can favorite.
    pub(crate) favorite_limit: Option<i64>,
    /// The maximum tag query limit, the amount amount of tags a user can search.
    pub(crate) tag_query_limit: Option<i64>,
}
