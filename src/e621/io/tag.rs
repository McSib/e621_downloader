extern crate failure;
extern crate reqwest;

use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::Path;

use failure::Error;
use reqwest::{header::USER_AGENT, Client};

use crate::e621::data_sets::TagEntry;
use crate::e621::io::emergency_exit;
use crate::e621::USER_AGENT_VALUE;

/// Constant of the tag file's name.
pub static TAG_NAME: &'static str = "tags.txt";
static TAG_FILE_EXAMPLE: &'static str = include_str!("tags.txt");

/// A tag that can be either general or special.
#[derive(Debug, Clone)]
pub enum Tag {
    General(String),
    Special(String),
    None,
}

/// Tag object used for searching e621.
#[derive(Debug, Clone)]
pub enum Parsed {
    Pool(String),
    Set(String),
    Post(String),
    General(Tag),
}

/// Group object generated from parsed code.
#[derive(Debug, Clone)]
pub struct Group {
    pub name: String,
    pub tags: Vec<Parsed>,
}

/// Creates tag file if it doesn't exist.
pub fn create_tag_file(p: &Path) -> Result<(), Error> {
    if !p.exists() {
        let mut file = File::create(p)?;
        file.write_all(TAG_FILE_EXAMPLE.as_bytes())?;

        emergency_exit("The tag file is created, I recommend closing the application to include the artist you wish to download.");
    }

    Ok(())
}

/// Creates instance of the parser and parses groups and tags.
pub fn parse_tag_file(p: &Path) -> Result<Vec<Group>, Error> {
    let source = read_to_string(p)?;
    Ok(Parser {
        pos: 0,
        input: source,
    }
    .parse_groups()?)
}

pub struct TagIdentifier {
    identifier_client: Client,
}

impl TagIdentifier {
    fn new() -> Self {
        TagIdentifier {
            identifier_client: Client::new(),
        }
    }

    fn id_tag(tags: &str) -> Result<Tag, Error> {
        let identifier = TagIdentifier::new();
        let tag_type = identifier.search_for_tag(tags)?;
        Ok(tag_type)
    }

    fn search_for_tag(&self, tags: &str) -> Result<Tag, Error> {
        let tag_url = "https://e621.net/tag/index.json";
        let mut split: Vec<&str> = tags.split(' ').collect();
        split.retain(|elem| !elem.contains(':') && !elem.starts_with('-'));

        let mut tag_type = Tag::None;
        for tag in &split {
            let tag_entry: Vec<TagEntry> = self
                .identifier_client
                .get(tag_url)
                .header(USER_AGENT, USER_AGENT_VALUE)
                .query(&[("name", tag)])
                .send()?
                .json()?;

            // To ensure that the tag set is not empty
            if !tag_entry.is_empty() {
                // Grab the closest matching tag
                let tag = &tag_entry[0];
                let tag_count = tag.count;
                let tags_string = tags.to_string();
                tag_type = match tag.tag_type {
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
                };

                if let Tag::Special(_) = tag_type {
                    break;
                }
            } else {
                println!("Error: JSON Return for tag is empty!");
                println!("Info: The tag is either invalid or the tag is an alias.");
                println!("Info: Please use the proper tag for the program to work correctly.");
                emergency_exit(format!("The server was unable to find tag: {}!", tag).as_str());
            }
        }

        Ok(tag_type)
    }
}

/// Parser that reads a tag file and parses the tags.
struct Parser {
    /// Current cursor position in the array of characters.
    pos: usize,
    /// Input used for parsing.
    input: String,
}

impl Parser {
    /// Parses groups.
    pub fn parse_groups(&mut self) -> Result<Vec<Group>, Error> {
        let mut groups: Vec<Group> = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }

            if self.check_and_parse_comment() {
                continue;
            }

            if self.starts_with("[") {
                groups.push(self.parse_group()?);
            } else {
                bail!(format_err!("Tags can't be outside of groups!"));
            }
        }

        Ok(groups)
    }

    pub fn parse_group(&mut self) -> Result<Group, Error> {
        assert_eq!(self.consume_char(), '[');
        let group_name = self.consume_while(valid_group);
        assert_eq!(self.consume_char(), ']');

        let parsed_tags = match group_name.as_str() {
            "artists" | "general" => self.parse_tags(|parser| {
                let tag = parser.consume_while(valid_tag);
                Ok(Parsed::General(TagIdentifier::id_tag(&tag.trim())?))
            })?,
            "pools" => self.parse_tags(|parser| {
                let tag = parser.consume_while(valid_id);
                Ok(Parsed::Pool(tag))
            })?,
            "sets" => self.parse_tags(|parser| {
                let tag = parser.consume_while(valid_id);
                Ok(Parsed::Set(tag))
            })?,
            "single-post" => self.parse_tags(|parser| {
                let tag = parser.consume_while(valid_id);
                Ok(Parsed::Post(tag))
            })?,
            _ => bail!(format_err!("{} is an invalid group name!", group_name)),
        };

        Ok(Group {
            name: group_name,
            tags: parsed_tags,
        })
    }

    /// Parses tags.
    pub fn parse_tags<F>(&mut self, mut parse_method: F) -> Result<Vec<Parsed>, Error>
    where
        F: FnMut(&mut Self) -> Result<Parsed, Error>,
    {
        let mut tags: Vec<Parsed> = Vec::new();
        while !self.eof() {
            self.consume_whitespace();
            if self.check_and_parse_comment() {
                continue;
            }

            if self.starts_with("[") {
                break;
            }

            tags.push(parse_method(self)?);
        }

        Ok(tags)
    }

    /// Checks if next character is comment identifier and parses it if it is.
    fn check_and_parse_comment(&mut self) -> bool {
        if self.starts_with("#") {
            self.parse_comment();
            return true;
        }

        false
    }

    /// Parses tag from input.
    //    fn parse_tag(&mut self) -> Result<Parsed, Error> {
    //        let tag = self.consume_while(valid_tag);
    //        Ok(Parsed::General(TagIdentifier::id_tag(&tag.trim())?))
    //    }

    /// Skips over comment.
    fn parse_comment(&mut self) {
        self.consume_while(valid_comment);
    }

    /// Consume and discard zero or more whitespace characters.
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// Consumes characters until `test` returns false.
    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }

        result
    }

    /// Returns current char and pushes `self.pos` to the next char.
    fn consume_char(&mut self) -> char {
        let mut iter = self.get_current_input().char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        cur_char
    }

    /// Read the current char without consuming it.
    fn next_char(&mut self) -> char {
        self.get_current_input().chars().next().unwrap()
    }

    /// Checks if the current input starts with the given string.
    fn starts_with(&self, s: &str) -> bool {
        self.get_current_input().starts_with(s)
    }

    /// Gets current input from current `pos` onward.
    fn get_current_input(&self) -> &str {
        &self.input[self.pos..]
    }

    /// Checks whether or not `pos` is at end of file.
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}

/// Validates character for tag.
fn valid_tag(c: char) -> bool {
    match c {
        ' '...'\"' | '$'...'~' => true,
        _ => false,
    }
}

/// Validates character for id.
fn valid_id(c: char) -> bool {
    match c {
        '0'...'9' => true,
        _ => false,
    }
}

/// Validates character for group
fn valid_group(c: char) -> bool {
    match c {
        'A'...'Z' => true,
        'a'...'z' => true,
        '-' => true,
        _ => false,
    }
}

/// Validates character for comment.
fn valid_comment(c: char) -> bool {
    match c {
        ' '...'~' => true,
        _ => false,
    }
}
