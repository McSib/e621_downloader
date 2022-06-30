use std::fs::{
    read_to_string,
    write,
};

use failure::{
    Error,
    ResultExt,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    from_str,
    to_string_pretty,
};

use crate::e621::io::emergency_exit;
use crate::e621::sender::entries::{
    PoolEntry,
    PostEntry,
    SetEntry,
    TagEntry,
};
use crate::e621::sender::RequestSender;

// TODO: Implement a special character section for character tags

/// Constant of the tag file's name.
pub const TAG_NAME: &str = "tags.json";
pub const TAG_FILE_EXAMPLE: &str = include_str!("tags.json");

pub enum IdTagSearchType {
    Single,
    Set,
    Pool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StringTag {
    name: String,
    validated: bool,
}

impl StringTag {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IdTag {
    id: u32,
    validated: bool,
}

impl IdTag {
    pub fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserTags {
    artists: Vec<StringTag>,
    pools: Vec<IdTag>,
    sets: Vec<IdTag>,
    single_posts: Vec<IdTag>,
    general: Vec<StringTag>,
}

impl UserTags {
    pub fn artists(&self) -> &Vec<StringTag> {
        &self.artists
    }

    pub fn pools(&self) -> &Vec<IdTag> {
        &self.pools
    }

    pub fn sets(&self) -> &Vec<IdTag> {
        &self.sets
    }

    pub fn single_posts(&self) -> &Vec<IdTag> {
        &self.single_posts
    }

    pub fn general(&self) -> &Vec<StringTag> {
        &self.general
    }
}

/// Creates instance of the parser and parses groups and tags.
pub fn parse_tag_file(request_sender: &RequestSender) -> Result<UserTags, Error> {
    let tag_validator = TagValidator::new(&request_sender);
    let mut user_tags: UserTags = from_str(
        &read_to_string(TAG_NAME)
            .with_context(|e| {
                error!("Unable to read tag file!");
                trace!("Possible I/O block when trying to read tag file...");
                format!("{}", e)
            })
            .unwrap(),
    )
    .unwrap();

    tag_validator.validate_user_tags(&mut user_tags);
    let json = to_string_pretty(&user_tags).unwrap();
    write(TAG_NAME, &json).unwrap();

    Ok(user_tags)
}

pub struct TagValidator {
    request_sender: RequestSender,
}

impl TagValidator {
    pub fn new(request_sender: &RequestSender) -> Self {
        TagValidator {
            request_sender: request_sender.clone(),
        }
    }

    pub fn validate_user_tags(&self, user_tags: &mut UserTags) {
        self.validate_string_tags(&mut user_tags.artists);
        self.validate_id_tags(&mut user_tags.single_posts, IdTagSearchType::Single);
        self.validate_id_tags(&mut user_tags.sets, IdTagSearchType::Set);
        self.validate_id_tags(&mut user_tags.pools, IdTagSearchType::Pool);
        self.validate_string_tags(&mut user_tags.general);
    }

    pub fn validate_string_tags(&self, tags: &mut Vec<StringTag>) {
        for tag in tags {
            if !tag.validated {
                self.search_for_tag(tag);

                if !tag.validated {
                    emergency_exit(&format!(
                        "Tag {} is not valid! Please ensure the tag is typed in correctly.",
                        tag.name
                    ));
                }
            }
        }
    }

    pub fn validate_id_tags(&self, tags: &mut Vec<IdTag>, id_type: IdTagSearchType) {
        for tag in tags {
            if !tag.validated {
                match id_type {
                    IdTagSearchType::Single => {
                        let is_post = self.request_sender.is_entry_from_appended_id::<PostEntry>(
                            &format!("{}", tag.id),
                            "single",
                        );

                        if is_post {
                            tag.validated = true;
                        }
                    }
                    IdTagSearchType::Set => {
                        let is_set = self
                            .request_sender
                            .is_entry_from_appended_id::<SetEntry>(&format!("{}", tag.id), "set");

                        if is_set {
                            tag.validated = true;
                        }
                    }
                    IdTagSearchType::Pool => {
                        let is_pool = self
                            .request_sender
                            .is_entry_from_appended_id::<PoolEntry>(&format!("{}", tag.id), "pool");

                        if is_pool {
                            tag.validated = true;
                        }
                    }
                }

                if !tag.validated {
                    emergency_exit(&format!(
                        "Tag {} is not valid! Please ensure the tag is typed in correctly.",
                        tag.id
                    ));
                }
            }
        }
    }

    /// Search for tag on e621.
    fn search_for_tag(&self, tag_search_term: &mut StringTag) {
        for tag in tag_search_term.name.clone().split(' ') {
            let temp = tag.trim_start_matches('-');
            match self.request_sender.get_tags_by_name(temp).first() {
                Some(_) => tag_search_term.validated = true,
                None => {
                    if let Some(alias_tag) = self.get_tag_from_alias(temp) {
                        tag_search_term.name = alias_tag.name;
                        tag_search_term.validated = true;
                    } else if temp.contains(':') {
                        tag_search_term.validated = true;
                    } else {
                        self.exit_tag_failure(temp);
                        unreachable!();
                    }
                }
            }
        }
    }

    /// Checks if the tag is an alias and searches for the tag it is aliased to, returning it.
    fn get_tag_from_alias(&self, tag: &str) -> Option<TagEntry> {
        let entry = match self.request_sender.query_aliases(tag) {
            Some(e) => e.first().unwrap().clone(),
            None => {
                return None;
            }
        };

        // Is there possibly a way to make this better?
        Some(
            self.request_sender
                .get_tags_by_name(&entry.consequent_name)
                .first()
                .unwrap()
                .clone(),
        )
    }

    /// Emergency exits if a tag isn't identified.
    fn exit_tag_failure(&self, tag: &str) {
        error!("{} is invalid!", tag);
        info!("The tag may be a typo, be sure to double check and ensure that the tag is correct.");
        emergency_exit(format!("The server API call was unable to find tag: {}!", tag).as_str());
    }
}