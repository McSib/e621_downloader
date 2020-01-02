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

trait UnwrapOrFail<T> {
    fn unwrap_or_fail<F>(self, closure: F) -> T
    where
        F: FnOnce();
}

impl<T> UnwrapOrFail<T> for Option<T> {
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
#[derive(Debug, Clone)]
pub enum Tag {
    /// A general tag that is used for everything except artist and sometimes character (depending on the amount of posts tied to it)
    General(String),
    /// A special tag that is searched differently from general tags (artist and characters).
    Special(String),
    /// This is used only if the type of tag is `2` or its greater than `5`.
    None,
}

/// Tag object used for searching e621.
#[derive(Debug, Clone)]
pub enum Parsed {
    /// Pool containing its ID
    Pool(String),
    /// Set containing its ID
    Set(String),
    /// A single post containing its ID
    Post(String),
    /// A general tag contain the [`Tag`] object.
    General(Tag),
}

/// Group object generated from parsed code.
#[derive(Debug, Clone)]
pub struct Group {
    /// Name of group
    pub name: String,
    /// [`Vec`] of [`Parsed`] tags
    pub tags: Vec<Parsed>,
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
pub fn parse_tag_file(p: &Path, request_sender: RequestSender) -> Result<Vec<Group>, Error> {
    let source = read_to_string(p)?;
    Ok(TagParser {
        parser: Parser {
            pos: 0,
            input: source,
        },
        request_sender,
    }
    .parse_groups()?)
}

/// Identifier to help categorize tags.
pub struct TagIdentifier {
    request_sender: RequestSender,
}

impl TagIdentifier {
    /// Creates new identifier.
    fn new(request_sender: RequestSender) -> Self {
        TagIdentifier { request_sender }
    }

    /// Identifies tag and returns [`Tag`].
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
        let mut split: Vec<&str> = tags.split(' ').collect();
        split.retain(|elem| !elem.contains(':') && !elem.starts_with('-'));

        let mut tag_type = Tag::None;
        for tag in split {
            let tag_entries: Vec<TagEntry> = self.request_sender.get_tags_by_name(tag)?;

            // To ensure that the tag set is not empty
            if !tag_entries.is_empty() {
                tag_type = self.process_tag_data(tags, &tag_entries[0]);
            } else {
                let alias_entry = self.is_alias(&tag)?;
                tag_type = self.process_tag_data(tags, &alias_entry);
            }

            if let Tag::Special(_) = tag_type {
                break;
            }
        }

        Ok(tag_type)
    }

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

        bail!(format_err!("Tag entry was not found!"))
    }

    fn exit_tag_failure(&self, tag: &str) {
        println!("Error: JSON Return for tag is empty!");
        println!("Info: The tag is either invalid or the tag is an alias.");
        println!("Info: Please use the proper tag for the program to work correctly.");
        emergency_exit(format!("The server was unable to find tag: {}!", tag).as_str());
    }

    fn process_tag_data(&self, tags: &str, tag_entry: &TagEntry) -> Tag {
        // Grab the closest matching tag
        let tag_count = tag_entry.count;
        let tags_string = tags.to_string();
        match tag_entry.tag_type {
            // `0`: General; `3`: Copyright; `5`: Species;
            0 | 3 | 5 => Tag::General(tags_string),
            // `4`: Character;
            4 => {
                if tag_count > 1500 {
                    Tag::General(tags_string)
                } else {
                    Tag::Special(tags_string)
                }
            }
            // `1`: Artist;
            1 => Tag::Special(tags_string),
            _ => Tag::None,
        }
    }
}

/// Parser that reads a tag file and parses the tags.
struct TagParser {
    parser: Parser,
    request_sender: RequestSender,
}

impl TagParser {
    /// Parses groups.
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

    // Parses single group and all tags for it.
    pub fn parse_group(&mut self) -> Result<Group, Error> {
        assert_eq!(self.parser.consume_char(), '[');
        let group_name = self.parser.consume_while(valid_group);
        assert_eq!(self.parser.consume_char(), ']');

        //        self.parse_tags(&group_name)?;

        Ok(Group {
            name: group_name,
            tags: parsed_tags,
        })
    }

    fn parse_pool(&mut self) -> Result<Parsed, Error> {
        let tag = self.parser.consume_while(valid_id);
        Ok(Parsed::Pool(tag))
    }

    fn parse_set(&mut self) -> Result<Parsed, Error> {
        let tag = self.parser.consume_while(valid_id);
        Ok(Parsed::Set(tag))
    }

    fn parse_post(&mut self) -> Result<Parsed, Error> {
        let tag = self.parser.consume_while(valid_id);
        Ok(Parsed::Post(tag))
    }

    fn parse_general(&mut self) -> Result<Parsed, Error> {
        let tag = self.parser.consume_while(valid_tag);
        Ok(Parsed::General(TagIdentifier::id_tag(
            &tag.trim(),
            self.request_sender.clone(),
        )?))
    }

    /// Parses tags.
    pub fn parse_tags(&mut self) -> Result<Vec<Parsed>, Error> {
        //        let mut tags: Vec<Parsed> = Vec::new();
        //        loop {
        //            self.parser.consume_whitespace();
        //            if self.check_and_parse_comment() {
        //                continue;
        //            }
        //
        //            if self.parser.starts_with("[") {
        //                break;
        //            }
        //
        //            if self.parser.eof() {
        //                break;
        //            }
        //
        //            tags.push();
        //        }
        //
        //        Ok(tags)
        // TODO: Implement a way for the tags to parse to the correct category without the use of closures.
        unimplemented!()
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

/// Validates character for id.
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
