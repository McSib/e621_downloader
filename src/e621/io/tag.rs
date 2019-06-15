extern crate serde;
extern crate serde_json;

use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use serde::Deserialize;

use crate::e621::io::emergency_exit;

use super::super::reqwest::Client;
use std::process::exit;
use core::borrow::BorrowMut;

/// Constant of the tag file's name.
pub static TAG_NAME: &'static str = "tags.txt";
static TAG_FILE_EXAMPLE: &'static str = include_str!("tags.txt");

/// All the type the tag can be (this will help in identifying how to treat the tag).
#[derive(Debug, Clone)]
pub enum TagType {
    None,
    General,
    Artist,
    Copyright,
    Character,
    Species,
}

#[derive(Deserialize, Clone)]
pub struct TagJsonData {
    id: u32,
    name: String,
    count: u32,
    #[serde(rename = "type")]
    tag_type: u8,
    type_locked: Option<bool>,
}

/// Tag object used for searching e621.
#[derive(Debug, Clone)]
pub struct Tag {
    /// Value of the tag.
    pub value: String,
    /// The type of tag it is.
    pub tag_type: TagType,
}

/// Creates tag file if it doesn't exist.
pub fn create_tag_file(p: &Path) -> Result<(), Box<Error>> {
    if !p.exists() {
        let mut file = File::create(p)?;
        file.write(TAG_FILE_EXAMPLE.as_bytes())?;

        emergency_exit("The tag file is created, I recommend closing the application to include the artist you wish to download.");
    }

    Ok(())
}

/// Creates instance of the parser and parses groups and tags.
pub fn parse_tag_file(p: &Path) -> Result<Vec<Tag>, Box<Error>> {
    let source = read_to_string(p)?;
    Ok(Parser { pos: 0, input: source }.parse_tags())
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
    pub fn parse_tags(&mut self) -> Vec<Tag> {
        let mut tags: Vec<Tag> = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }

            if self.check_and_parse_comment() {
                continue;
            }

            tags.push(self.parse_tag());
        }

        tags
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
    fn parse_tag(&mut self) -> Tag {
        let tag_value = self.consume_while(valid_tag);

        Tag {
            value: tag_value.trim_end_matches(' ').to_string(),
            tag_type: TagType::None
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
        where F: Fn(char) -> bool {
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
        _ => false
    }
}

/// Validates character for comment.
fn valid_comment(c: char) -> bool {
    match c {
        ' '...'~' => true,
        _ => false
    }
}

/// Validates tags parsed from the parser.
pub struct TagValidator {
}

impl TagValidator {
    /// Constructs new instance of `TagValidator`
    pub fn new() -> TagValidator {
        TagValidator {}
    }

    pub fn validate_and_identify_tags(&mut self, tags: &mut Vec<Tag>) -> Result<(), Box<Error>> {
        let url = "https://e621.net/tag/index.json";
        let client = Client::new();
        for tag in tags.iter_mut() {
            let collected_tag_data: Vec<TagJsonData> = client.get(url)
                                                    .query(&[("name", tag.value.clone())])
                                                    .send()?
                .json()?;
            if collected_tag_data.is_empty() {
                println!("{} is invalid!", tag.value);
                println!("Validation failed!");
                sleep(Duration::from_secs(2));
                exit(-1);
            }

            let data = &collected_tag_data[0];
            tag.tag_type = self.get_tag_type(&data.tag_type);
        }

        Ok(())
    }

    fn get_tag_type(&self, tag_data: &u8) -> TagType {
        match tag_data {
            0 => return TagType::General,
            1 => return TagType::Artist,
            2 => return TagType::Copyright,
            3 => return TagType::Character,
            4 => return TagType::Species,
            _ => return TagType::None,
        }
    }
}