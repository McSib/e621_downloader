use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::{stdin, Write};
use std::path::Path;
use std::process::exit;

/// Constant of the tag file's name.
pub static TAG_NAME: &'static str = "tags.txt";
static TAG_FILE_EXAMPLE: &'static str = include_str!("tags.txt");

/// Tag object used for searching e621.
pub struct Tag {
    /// Value of the tag.
    pub value: String,
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

/// Exits the program after message explaining the error and prompting the user to press `ENTER`.
fn emergency_exit(error: &str) {
    println!("{}", error);
    println!("Press ENTER to close the application...");
    let mut line = String::new();
    stdin().read_line(&mut line).unwrap_or_default();
    exit(0);
}

/// Creates instance of the parser and parses tags.
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
    /// Parses input.
    pub fn parse_tags(&mut self) -> Vec<Tag> {
        let mut tags: Vec<Tag> = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }

            if self.starts_with("#") {
                self.parse_comment();
            } else {
                tags.push(self.parse_tag());
            }
        }

        tags
    }

    /// Parses tag from input.
    fn parse_tag(&mut self) -> Tag {
        let tag_value = self.consume_while(|c| match c {
            ' '...'~' => true,
            _ => false
        });

        Tag {
            value: tag_value,
        }
    }

    /// Skips over comment.
    fn parse_comment(&mut self) {
        self.consume_while(|c| match c {
            'a'...'z' | 'A'...'Z' | '0'...'9' | ' '...'/' | ':'...'@' | '['...'`' | '{'...'~' => true,
            _ => false
        });
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
