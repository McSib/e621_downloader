use std::error::Error;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::Path;

use crate::e621::io::emergency_exit;

/// Constant of the tag file's name.
pub static TAG_NAME: &'static str = "tags.txt";
static TAG_FILE_EXAMPLE: &'static str = include_str!("tags.txt");

/// Tag object used for searching e621.
#[derive(Debug, Clone)]
pub enum Parsed {
    Pool(String),
    Set(String),
    Post(String),
    General(String),
}

/// Creates tag file if it doesn't exist.
pub fn create_tag_file(p: &Path) -> Result<(), Box<Error>> {
    if !p.exists() {
        let mut file = File::create(p)?;
        file.write_all(TAG_FILE_EXAMPLE.as_bytes())?;

        emergency_exit("The tag file is created, I recommend closing the application to include the artist you wish to download.");
    }

    Ok(())
}

/// Creates instance of the parser and parses groups and tags.
pub fn parse_tag_file(p: &Path) -> Result<Vec<Parsed>, Box<Error>> {
    let source = read_to_string(p)?;
    Ok(Parser {
        pos: 0,
        input: source,
    }
    .parse_tags())
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
    pub fn parse_tags(&mut self) -> Vec<Parsed> {
        let mut tags: Vec<Parsed> = Vec::new();
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
    fn parse_tag(&mut self) -> Parsed {
        let tag_id = self.consume_while(valid_tag);
        self.consume_whitespace();

        let tag_type = if self.starts_with(":") {
            self.consume_char();
            self.consume_whitespace();
            self.consume_while(valid_type)
        } else {
            String::new()
        };

        match tag_type.trim() {
            "pool" => Parsed::Pool(tag_id),
            "set" => Parsed::Set(tag_id),
            "post" => Parsed::Post(tag_id),
            _ => Parsed::General(tag_id.trim_end_matches(' ').to_string()),
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
        ' '...'\"' | '$'...'9' | ';'...'~' => true,
        _ => false,
    }
}

/// Validates character for tag.
fn valid_type(c: char) -> bool {
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
