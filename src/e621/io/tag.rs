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

use std::fs::read_to_string;

use anyhow::{Context, Error};

use crate::e621::io::emergency_exit;
use crate::e621::io::parser::BaseParser;
use crate::e621::sender::entries::TagEntry;
use crate::e621::sender::RequestSender;

/// Constant of the tag file's name.
pub(crate) const TAG_NAME: &str = "tags.txt";

/// An example file for newly created tag files.
pub(crate) const TAG_FILE_EXAMPLE: &str = include_str!("tags.txt");

/// A tag that can be either general or special.
#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub(crate) enum TagSearchType {
    /// A general tag that is used for everything except artist and sometimes character (depending on the amount of posts tied to it)
    General,
    /// A special tag that is searched differently from general tags (artist and characters).
    Special,
    /// This is used only if the type of tag is `2` or its greater than `5`.
    None,
}

/// The type a tag can be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TagType {
    Pool,
    Set,
    General,
    Artist,
    Post,
    Unknown,
}

/// A tag that contains its name, search type, and tag type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tag {
    /// The name of the tag.
    name: String,
    /// The search type of the tag.
    search_type: TagSearchType,
    /// The tag type of the tag.
    tag_type: TagType,
}

impl Tag {
    fn new(tag: &str, category: TagSearchType, tag_type: TagType) -> Self {
        Tag {
            name: String::from(tag),
            search_type: category,
            tag_type,
        }
    }

    /// The name of the tag.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The search type of the tag.
    pub(crate) fn search_type(&self) -> &TagSearchType {
        &self.search_type
    }

    /// The tag type of the tag.
    pub(crate) fn tag_type(&self) -> &TagType {
        &self.tag_type
    }
}

impl Default for Tag {
    fn default() -> Self {
        Tag {
            name: String::new(),
            search_type: TagSearchType::None,
            tag_type: TagType::Unknown,
        }
    }
}

/// Group object generated from parsed code.
#[derive(Debug, Clone)]
pub(crate) struct Group {
    /// The name of group.
    name: String,
    /// A [Vec] containing all the tags parsed.
    tags: Vec<Tag>,
}

impl Group {
    pub(crate) fn new(name: String) -> Self {
        Group {
            name,
            tags: Vec::new(),
        }
    }

    /// The name of group.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// A [Vec] containing all the tags parsed.
    pub(crate) fn tags(&self) -> &Vec<Tag> {
        &self.tags
    }
}

/// Parses the tag file and returns the serialized form of it.
///
/// # Arguments
///
/// * `request_sender`: The sender to use for the API call (this is used for tag and alias checks).
///
/// returns: Result<Vec<Group, Global>, Error>
pub(crate) fn parse_tag_file(request_sender: &RequestSender) -> Result<Vec<Group>, Error> {
    TagParser {
        parser: BaseParser::new(
            read_to_string(TAG_NAME)
                .with_context(|| {
                    error!("Unable to read tag file!");
                    "Possible I/O block when trying to read tag file..."
                })
                .unwrap(),
        ),
        request_sender: request_sender.clone(),
    }
    .parse_groups()
}

/// Identifier to help categorize tags.
pub(crate) struct TagIdentifier {
    /// Request sender for making any needed API calls.
    request_sender: RequestSender,
}

impl TagIdentifier {
    /// Creates new identifier.
    fn new(request_sender: RequestSender) -> Self {
        TagIdentifier { request_sender }
    }

    /// Identifies tags to ensure they exist.
    ///
    /// # Arguments
    ///
    /// * `tags`: Tags to id.
    /// * `request_sender`: The sender to use for the API calls.
    ///
    /// returns: Tag
    fn id_tag(tags: &str, request_sender: RequestSender) -> Tag {
        let identifier = TagIdentifier::new(request_sender);
        identifier.search_for_tag(tags)
    }

    /// Search for tag on e621.
    ///
    /// # Arguments
    ///
    /// * `tags`: Tags to search for.
    ///
    /// returns: Tag
    fn search_for_tag(&self, tags: &str) -> Tag {
        // Splits the tags and cycles through each one, checking if they are valid and searchable tags
        // If the tag isn't searchable, the tag will default and consider itself invalid. Which will
        // then be filtered through the last step.
        let mut map = tags
            .split(' ')
            .map(|e| {
                let temp = e.trim_start_matches('-');
                match self.request_sender.get_tags_by_name(temp).first() {
                    Some(entry) => self.create_tag(tags, entry),
                    None => {
                        if let Some(alias_tag) = self.get_tag_from_alias(temp) {
                            self.create_tag(tags, &alias_tag)
                        } else if temp.contains(':') {
                            Tag::default()
                        } else {
                            self.exit_tag_failure(temp);
                            unreachable!();
                        }
                    }
                }
            })
            .filter(|e| *e != Tag::default());

        // Tries to return any tag in the map with category special, return the last element otherwise.
        // If returning the last element fails, assume the tag is syntax only and default.
        map.find(|e| e.search_type == TagSearchType::Special)
            .unwrap_or_else(|| {
                map.last()
                    .unwrap_or_else(|| Tag::new(tags, TagSearchType::General, TagType::General))
            })
    }

    /// Checks if the tag is an alias and searches for the tag it is aliased to, returning it.
    ///
    /// # Arguments
    ///
    /// * `tag`: Alias to check for.
    ///
    /// returns: Option<TagEntry>
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
    ///
    /// # Arguments
    ///
    /// * `tag`: Tag to log for.
    fn exit_tag_failure(&self, tag: &str) {
        error!("{tag} is invalid!");
        info!("The tag may be a typo, be sure to double check and ensure that the tag is correct.");
        emergency_exit(format!("The server API call was unable to find tag: {tag}!").as_str());
    }

    /// Processes the tag type and creates the appropriate tag for it.
    ///
    /// # Arguments
    ///
    /// * `tags`: Tags to use.
    /// * `tag_entry`: The tag entry related to the tags.
    ///
    /// returns: Tag
    fn create_tag(&self, tags: &str, tag_entry: &TagEntry) -> Tag {
        let tag_type = tag_entry.to_tag_type();
        let category = match tag_type {
            TagType::General => {
                const CHARACTER_CATEGORY: u8 = 4;
                if tag_entry.category == CHARACTER_CATEGORY {
                    if tag_entry.post_count > 1500 {
                        TagSearchType::General
                    } else {
                        TagSearchType::Special
                    }
                } else {
                    TagSearchType::General
                }
            }
            TagType::Artist => TagSearchType::Special,
            _ => unreachable!(),
        };

        Tag::new(tags, category, tag_type)
    }
}

/// Parser that reads a tag file and parses the tags.
struct TagParser {
    /// Low-level parser for parsing raw data.
    parser: BaseParser,
    /// Request sender for any needed API calls.
    request_sender: RequestSender,
}

impl TagParser {
    /// Parses each group with all tags tied to them before returning a vector with all groups in it.
    pub(crate) fn parse_groups(&mut self) -> Result<Vec<Group>, Error> {
        let mut groups: Vec<Group> = Vec::new();
        loop {
            self.parser.consume_whitespace();
            if self.parser.eof() {
                break;
            }

            if self.check_and_parse_comment() {
                continue;
            }

            if self.parser.starts_with("[") {
                groups.push(self.parse_group());
            } else {
                self.parser.report_error("Tags must be in groups!");
            }
        }

        Ok(groups)
    }

    /// Parses a group and all tags tied to it before returning the result.
    fn parse_group(&mut self) -> Group {
        assert_eq!(self.parser.consume_char(), '[');
        let group_name = self.parser.consume_while(valid_group);
        assert_eq!(self.parser.consume_char(), ']');

        let mut group = Group::new(group_name);
        self.parse_tags(&mut group);

        group
    }

    /// Parses all tags for a group and stores it.
    ///
    /// # Arguments
    ///
    /// * `group`: The group to parse.
    fn parse_tags(&mut self, group: &mut Group) {
        let mut tags: Vec<Tag> = Vec::new();
        loop {
            self.parser.consume_whitespace();
            if self.check_and_parse_comment() {
                continue;
            }

            if self.parser.starts_with("[") {
                break;
            }

            if self.parser.eof() {
                break;
            }

            tags.push(self.parse_tag(group.name()));
        }

        group.tags = tags;
    }

    /// Parses a single tag and identifies it before returning the result.
    ///
    /// # Arguments
    ///
    /// * `group_name`:  Group name to parse the tag for.
    ///
    /// returns: Tag
    fn parse_tag(&mut self, group_name: &str) -> Tag {
        match group_name {
            "artists" | "general" => {
                let tag = self.parser.consume_while(valid_tag);
                TagIdentifier::id_tag(tag.trim(), self.request_sender.clone())
            }
            e => {
                let tag = self.parser.consume_while(valid_id);
                let tag_type = match e {
                    "pools" => TagType::Pool,
                    "sets" => TagType::Set,
                    "single-post" => TagType::Post,
                    _ => {
                        self.parser.report_error("Unknown tag type!");
                        TagType::Unknown
                    }
                };

                Tag::new(tag.trim(), TagSearchType::Special, tag_type)
            }
        }
    }

    /// Checks if next character is comment identifier and parses it if it is.
    fn check_and_parse_comment(&mut self) -> bool {
        if self.parser.starts_with("#") {
            self.parse_comment();
            return true;
        }

        false
    }

    /// Skips over comment.
    fn parse_comment(&mut self) {
        self.parser.consume_while(valid_comment);
    }
}

/// Validates character for tag.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_tag(c: char) -> bool {
    match c {
        ' '..='\"' | '$'..='~' => true,
        // This will check for any special characters in the validator.
        _ => {
            if c != '#' {
                return c.is_alphanumeric();
            }

            false
        }
    }
}

/// Validates character for id.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_id(c: char) -> bool {
    c.is_ascii_digit()
}

/// Validates character for group
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_group(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z' | '-')
}

/// Validates character for comment.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_comment(c: char) -> bool {
    match c {
        ' '..='~' => true,
        _ => c.is_alphanumeric(),
    }
}
