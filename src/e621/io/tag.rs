extern crate failure;

use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::Path;

use failure::Error;

use crate::e621::io::emergency_exit;
use crate::e621::io::parser::Parser;
use crate::e621::sender::{RequestSender, TagEntry, ToTagType};

/// Constant of the tag file's name.
pub const TAG_NAME: &str = "tags.txt";
const TAG_FILE_EXAMPLE: &str = include_str!("tags.txt");

/// Returns `T` if it isn't an error. If it is, it will run a closure that is expected to panic.
trait UnwrapOrFail<T> {
    fn unwrap_or_fail<F>(self, closure: F) -> T
    where
        F: FnOnce();
}

impl<T> UnwrapOrFail<T> for Option<T> {
    /// Attempts to unwrap and return `T`. If `None`, it will run a closure that is expected to panic.
    ///
    /// # Panics
    /// Will panic with `unreachable!()` if the closure does not panic itself.
    fn unwrap_or_fail<F>(self, closure: F) -> T
    where
        F: FnOnce(),
    {
        match self {
            Some(e) => e,
            None => {
                closure();
                unreachable!()
            }
        }
    }
}

/// A tag that can be either general or special.
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum TagCategory {
    /// A general tag that is used for everything except artist and sometimes character (depending on the amount of posts tied to it)
    General,
    /// A special tag that is searched differently from general tags (artist and characters).
    Special,
    /// This is used only if the type of tag is `2` or its greater than `5`.
    None,
}

/// The type a tag can be.
#[derive(Debug, Clone)]
pub enum TagType {
    Pool,
    Set,
    General,
    Artist,
    Post,
    None,
}

/// A tag that contains its name, search type, and tag type.
#[derive(Debug, Clone)]
pub struct Tag {
    /// The name of the tag.
    pub name: String,
    /// The search type of the tag.
    pub search_type: TagCategory,
    /// The tag type of the tag.
    pub tag_type: TagType,
}

impl Tag {
    fn new(tag: &str, category: TagCategory, tag_type: TagType) -> Self {
        Tag {
            name: String::from(tag),
            search_type: category,
            tag_type,
        }
    }
}

impl Default for Tag {
    fn default() -> Self {
        Tag {
            name: String::new(),
            search_type: TagCategory::None,
            tag_type: TagType::None,
        }
    }
}

/// Group object generated from parsed code.
#[derive(Debug, Clone)]
pub struct Group {
    /// The name of group.
    pub name: String,
    /// A `Vec` containing all the tags parsed.
    pub tags: Vec<Tag>,
}

impl Group {
    pub fn new(name: String) -> Self {
        Group {
            name,
            tags: Vec::new(),
        }
    }
}

/// Creates tag file if it doesn't exist.
pub fn create_tag_file() -> Result<(), Error> {
    let tag_path = Path::new(TAG_NAME);
    if !tag_path.exists() {
        let mut file = File::create(tag_path)?;
        file.write_all(TAG_FILE_EXAMPLE.as_bytes())?;

        emergency_exit(
            "The tag file is created, the application will close so you can include \
             the artists, sets, pools, and individual posts you wish to download.",
        );
    }

    Ok(())
}

/// Creates instance of the parser and parses groups and tags.
pub fn parse_tag_file(request_sender: &RequestSender) -> Result<Vec<Group>, Error> {
    Ok(TagParser {
        parser: Parser {
            pos: 0,
            input: read_to_string(TAG_NAME).expect("Unable to read tag file!"),
        },
        request_sender: request_sender.clone(),
    }
    .parse_groups()?)
}

/// Identifier to help categorize tags.
pub struct TagIdentifier {
    /// Request sender for making any needed API calls.
    request_sender: RequestSender,
}

impl TagIdentifier {
    /// Creates new identifier.
    fn new(request_sender: RequestSender) -> Self {
        TagIdentifier { request_sender }
    }

    /// Identifies tag and returns `Tag`.
    fn id_tag(tags: &str, request_sender: RequestSender) -> Tag {
        let identifier = TagIdentifier::new(request_sender);
        identifier.search_for_tag(tags)
    }

    /// Search for tag on e621.
    fn search_for_tag(&self, tag_str: &str) -> Tag {
        let tags: Vec<&str> = tag_str
            .split(' ')
            .filter(|elem| !elem.contains(':') && !elem.starts_with('-'))
            .collect();
        let mut temp_tag = Tag::default();
        for tag in tags {
            temp_tag = match self.request_sender.get_tags_by_name(tag).first() {
                Some(entry) => self.create_tag(tag_str, entry),
                None => self.create_tag(tag_str, &self.get_tag_from_alias(tag)),
            };

            if temp_tag.search_type == TagCategory::Special {
                break;
            }
        }

        temp_tag
    }

    /// Checks if the tag is an alias and searches for the tag it is aliased to, returning it.
    fn get_tag_from_alias(&self, tag: &str) -> TagEntry {
        let entry = match self.request_sender.query_aliases(tag) {
            None => {
                self.exit_tag_failure(tag);
                unreachable!()
            }
            Some(e) => e.first().unwrap().clone(),
        };
        // Is there possibly a way to make this better?
        self.request_sender
            .get_tags_by_name(entry.consequent_name.as_str())
            .first()
            .unwrap()
            .clone()
    }

    /// Emergency exits if a tag isn't identified.
    fn exit_tag_failure(&self, tag: &str) {
        println!("Error: JSON Return for tag is empty!");
        println!("Info: The tag may be a typo, be sure to double check and ensure that the tag is correct.");
        emergency_exit(format!("The server API call was unable to find tag: {}!", tag).as_str());
    }

    /// Processes the tag type and creates the appropriate tag for it.
    fn create_tag(&self, tags: &str, tag_entry: &TagEntry) -> Tag {
        let tag_type = tag_entry.to_tag_type();
        let category = match tag_type {
            TagType::General => {
                // Checks if the tag type is Character
                if tag_entry.category == 4 {
                    if tag_entry.post_count > 1500 {
                        TagCategory::General
                    } else {
                        TagCategory::Special
                    }
                } else {
                    TagCategory::General
                }
            }
            TagType::Artist => TagCategory::Special,
            _ => unreachable!(),
        };

        Tag::new(tags, category, tag_type)
    }
}

/// Parser that reads a tag file and parses the tags.
struct TagParser {
    /// Low-level parser for parsing raw data.
    parser: Parser,
    /// Request sender for any needed API calls.
    request_sender: RequestSender,
}

impl TagParser {
    /// Parses each group with all tags tied to them before returning a vector with all groups in it.
    pub fn parse_groups(&mut self) -> Result<Vec<Group>, Error> {
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
                bail!(format_err!("Tags can't be outside of groups!"));
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

            tags.push(self.parse_tag(&group.name));
        }

        group.tags = tags;
    }

    /// Parses a single tag and identifies it before returning the result.
    fn parse_tag(&mut self, group_name: &str) -> Tag {
        match group_name {
            "artists" | "general" => {
                let tag = self.parser.consume_while(valid_tag);
                TagIdentifier::id_tag(&tag, self.request_sender.clone())
            }
            e => {
                let tag = self.parser.consume_while(valid_id);
                let tag_type = match e {
                    "pools" => TagType::Pool,
                    "sets" => TagType::Set,
                    "single-post" => TagType::Post,
                    _ => unreachable!(),
                };

                Tag::new(&tag, TagCategory::Special, tag_type)
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

///// Validates character for id.
fn valid_id(c: char) -> bool {
    matches!(c, '0'..='9')
}

/// Validates character for group
fn valid_group(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z' | '-')
}

/// Validates character for comment.
fn valid_comment(c: char) -> bool {
    match c {
        ' '..='~' => true,
        _ => c.is_alphanumeric(),
    }
}
