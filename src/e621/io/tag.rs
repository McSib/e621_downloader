extern crate failure;

use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::Path;

use failure::Error;

use crate::e621::io::emergency_exit;
use crate::e621::io::parser::Parser;
use crate::e621::sender::{AliasEntry, RequestSender, TagEntry};

/// Constant of the tag file's name.
pub static TAG_NAME: &str = "tags.txt";
static TAG_FILE_EXAMPLE: &str = include_str!("tags.txt");

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

/// A tag that contains its raw name, search type, and tag type.
#[derive(Debug, Clone)]
pub struct Tag {
    /// The raw string of the tag.
    pub raw: String,
    /// The search type of the tag.
    pub search_type: TagCategory,
    /// The tag type of the tag.
    pub tag_type: TagType,
}

impl Tag {
    fn new(tag: &str, category: TagCategory, tag_type: TagType) -> Self {
        Tag {
            raw: String::from(tag),
            search_type: category,
            tag_type,
        }
    }
}

impl Default for Tag {
    fn default() -> Self {
        Tag {
            raw: String::new(),
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

/// Creates tag file if it doesn't exist.
pub fn create_tag_file(p: &Path) -> Result<(), Error> {
    if !p.exists() {
        let mut file = File::create(p)?;
        file.write_all(TAG_FILE_EXAMPLE.as_bytes())?;

        emergency_exit(
            "The tag file is created, the application will close so you can include \
             the artists, sets, pools, and individual posts you wish to download.",
        );
    }

    Ok(())
}

/// Creates instance of the parser and parses groups and tags.
pub fn parse_tag_file(p: &Path, request_sender: &RequestSender) -> Result<Vec<Group>, Error> {
    let source = read_to_string(p)?;
    Ok(TagParser {
        parser: Parser {
            pos: 0,
            input: source,
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
    fn id_tag(tags: &str, request_sender: RequestSender) -> Result<Tag, Error> {
        let identifier = TagIdentifier::new(request_sender);
        let tag_type = identifier.search_for_tag(tags)?;
        Ok(tag_type)
    }

    /// Search for tag on e621.
    ///
    /// # Important
    /// This does not identify an alias right now, that may change in the future.
    fn search_for_tag(&self, tags: &str) -> Result<Tag, Error> {
        let mut tag = Tag::default();
        let mut tags_split: Vec<&str> = tags.split(' ').collect();
        tags_split.retain(|elem| !elem.contains(':') && !elem.starts_with('-'));
        for tag_str in tags_split {
            let tag_entries: Vec<TagEntry> = self.request_sender.get_tags_by_name(tag_str)?;
            tag = match tag_entries.first() {
                Some(entry) => self.process_tag_data(tags, entry),
                None => self.process_tag_data(tags, &self.is_alias(tag_str)?),
            };

            if tag.search_type == TagCategory::Special {
                break;
            }
        }

        Ok(tag)
    }

    /// Checks if the tag is an alias and searches for the tag it is aliased to, returning it.
    fn is_alias(&self, tag: &str) -> Result<TagEntry, Error> {
        let alias_entries: Vec<AliasEntry> = self.request_sender.query_aliases(tag)?;
        let entry = alias_entries
            .first()
            .unwrap_or_fail(|| self.exit_tag_failure(&tag));
        if !entry.pending {
            let tag_entry: TagEntry = self
                .request_sender
                .get_tag_by_id(&format!("{}", entry.alias_id))?;
            return Ok(tag_entry);
        } else {
            self.exit_tag_failure(&tag);
        }

        unreachable!()
    }

    /// Emergency exits if a tag isn't identified.
    fn exit_tag_failure(&self, tag: &str) {
        println!("Error: JSON Return for tag is empty!");
        println!("Info: The tag is either invalid or the tag is an alias.");
        println!("Info: Please use the proper tag for the program to work correctly.");
        emergency_exit(format!("The server was unable to find tag: {}!", tag).as_str());
    }

    /// Processes the tag type and creates the appropriate tag for it.
    fn process_tag_data(&self, tags: &str, tag_entry: &TagEntry) -> Tag {
        // Grab the closest matching tag
        match tag_entry.tag_type {
            // `0`: General; `3`: Copyright; `5`: Species;
            0 | 3 | 5 => Tag::new(tags, TagCategory::General, TagType::General),
            // `4`: Character;
            4 => {
                if tag_entry.count > 1500 {
                    Tag::new(tags, TagCategory::General, TagType::General)
                } else {
                    Tag::new(tags, TagCategory::Special, TagType::General)
                }
            }
            // `1`: Artist;
            1 => Tag::new(tags, TagCategory::Special, TagType::Artist),
            _ => unreachable!(),
        }
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
                groups.push(self.parse_group()?);
            } else {
                bail!(format_err!("Tags can't be outside of groups!"));
            }
        }

        Ok(groups)
    }

    /// Parses a group and all tags tied to it before returning the result.
    fn parse_group(&mut self) -> Result<Group, Error> {
        assert_eq!(self.parser.consume_char(), '[');
        let group_name = self.parser.consume_while(valid_group);
        assert_eq!(self.parser.consume_char(), ']');

        let mut group = Group {
            name: group_name,
            tags: Vec::new(),
        };
        self.parse_tags(&mut group)?;

        Ok(group)
    }

    /// Parses all tags for a group and stores it.
    fn parse_tags(&mut self, group: &mut Group) -> Result<(), Error> {
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

            tags.push(self.parse_tag(&group.name)?);
        }

        group.tags = tags;
        Ok(())
    }

    /// Parses a single tag and identifies it before returning the result.
    fn parse_tag(&mut self, group_name: &str) -> Result<Tag, Error> {
        match group_name {
            "artists" | "general" => {
                let tag = self.parser.consume_while(valid_tag);
                Ok(TagIdentifier::id_tag(&tag, self.request_sender.clone())?)
            }
            "pools" => {
                let tag = self.parser.consume_while(valid_id);
                Ok(Tag::new(&tag, TagCategory::Special, TagType::Pool))
            }
            "sets" => {
                let tag = self.parser.consume_while(valid_id);
                Ok(Tag::new(&tag, TagCategory::Special, TagType::Set))
            }
            "single-post" => {
                let tag = self.parser.consume_while(valid_id);
                Ok(Tag::new(&tag, TagCategory::Special, TagType::Post))
            }
            _ => bail!(format_err!("Group name invalid!")),
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
    match c {
        '0'..='9' => true,
        _ => false,
    }
}

/// Validates character for group
fn valid_group(c: char) -> bool {
    match c {
        'A'..='Z' => true,
        'a'..='z' => true,
        '-' => true,
        _ => false,
    }
}

/// Validates character for comment.
fn valid_comment(c: char) -> bool {
    match c {
        ' '..='~' => true,
        _ => c.is_alphanumeric(),
    }
}
