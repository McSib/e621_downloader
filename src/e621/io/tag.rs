extern crate failure;
extern crate reqwest;

use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::Path;

use failure::Error;
use reqwest::{header::USER_AGENT, Client};

use super::serde_json::{from_value, Value};
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
pub fn parse_tag_file(p: &Path) -> Result<Vec<Parsed>, Error> {
    let source = read_to_string(p)?;
    Ok(Parser {
        pos: 0,
        input: source,
    }
    .parse_tags()?)
}

/// TODO: Implement this to remove the tag file type system.
pub struct TagIdentifier {
    identifier_client: Client,
}

impl TagIdentifier {
    fn new() -> Self {
        TagIdentifier {
            identifier_client: Client::new(),
        }
    }

    fn id_tag(tag: &String) -> Result<Tag, Error> {
        let tag_url = "https://e621.net/tag/index.json";
        let identifier = TagIdentifier::new();
        let tag_type = identifier.search_for_tag(tag, &tag_url)?;
        println!("{:?}", tag_type);
        Ok(tag_type)
    }

    fn search_for_tag(&self, tags: &String, url: &str) -> Result<Tag, Error> {
        let mut split: Vec<&str> = tags.split(' ').collect();
        split.retain(|elem| !elem.contains(':') && !elem.starts_with('-'));

        let mut tag_type = Tag::None;
        for tag in &split {
            let tag_entry: Value = self
                .identifier_client
                .get(url)
                .header(USER_AGENT, USER_AGENT_VALUE)
                .query(&[("name", tag)])
                .send()?
                .json()?;

            println!("{}", tag_entry.to_string());

            if tag_entry.to_string().contains("name") {
                let tag = &from_value::<Vec<TagEntry>>(tag_entry)?[0];
                let tag_count = tag.count;
                tag_type = match tag.tag_type {
                    0 | 3 | 5 => Tag::General(tags.clone()),
                    4 => {
                        if tag_count > 1500 {
                            Tag::General(tags.clone())
                        } else {
                            Tag::Special(tags.clone())
                        }
                    }
                    1 => Tag::Special(tags.clone()),
                    _ => Tag::None,
                };

                if let Tag::Special(_) = tag_type {
                    break;
                }
            } else {
                println!("Error: JSON Return for tag is empty!");
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
    pub fn parse_tags(&mut self) -> Result<Vec<Parsed>, Error> {
        let mut tags: Vec<Parsed> = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }

            if self.check_and_parse_comment() {
                continue;
            }

            tags.push(self.parse_tag()?);
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
    fn parse_tag(&mut self) -> Result<Parsed, Error> {
        let tag = self.consume_while(valid_tag);

        match "replace" {
            "pool" => Ok(Parsed::Pool(tag)),
            "set" => Ok(Parsed::Set(tag)),
            "post" => Ok(Parsed::Post(tag)),
            _ => Ok(Parsed::General(TagIdentifier::id_tag(
                &tag.trim().to_string(),
            )?)),
        }
    }

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

/// Validates character for comment.
fn valid_comment(c: char) -> bool {
    match c {
        ' '...'~' => true,
        _ => false,
    }
}
