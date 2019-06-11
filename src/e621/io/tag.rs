use std::error::Error;
use std::fs::{File, read_to_string};
use std::io::Write;
use std::path::Path;

use crate::e621::io::emergency_exit;
use std::thread::sleep;
use std::time::Duration;

/// Constant of the tag file's name.
pub static TAG_NAME: &'static str = "tags.txt";
static TAG_FILE_EXAMPLE: &'static str = include_str!("tags.txt");

/// Tag object used for searching e621.
#[derive(Debug, Clone)]
pub struct Tag {
    /// Value of the tag.
    pub value: String,
}

/// Groups contains tags given to it.
#[derive(Debug, Clone)]
pub struct Group {
    /// Name of group
    pub group_name: String,
    /// Tags in group
    pub tags: Vec<Tag>,
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
pub fn parse_tag_file(p: &Path) -> Result<Vec<Group>, Box<Error>> {
    let source = read_to_string(p)?;
    Ok(Parser { pos: 0, input: source }.parse_groups())
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
    pub fn parse_groups(&mut self) -> Vec<Group> {
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
                groups.push(self.parse_group());
            } else {
                println!("Tags can't be outside of a group!");
            }
        }

        groups
    }

    /// Parses group in input.
    fn parse_group(&mut self) -> Group {
        assert_eq!(self.consume_char(), '[');
        let name = self.get_group_name();
        self.consume_whitespace();

        assert_eq!(self.consume_char(), ']');
        let tags = self.parse_tags();

        Group {
            group_name: name,
            tags,
        }
    }

    /// Gets group name.
    fn get_group_name(&mut self) -> String {
        self.consume_while(valid_group_name)
    }

    /// Parses tags in group.
    fn parse_tags(&mut self) -> Vec<Tag> {
        let mut tags: Vec<Tag> = Vec::new();

        while !self.eof() {
            self.consume_whitespace();
            if self.check_and_parse_comment() {
                continue;
            }

            if self.starts_with("[") {
                break;
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

/// Validates character for group name.
fn valid_group_name(c: char) -> bool {
    match c {
        '!'...'\\' | '^'...'~' => true,
        _ => false
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
    groups: Vec<Group>,
    tags: Vec<Tag>,
}

impl TagValidator {
    /// Constructs new instance of `TagValidator`
    pub fn new(groups: &Vec<Group>) -> TagValidator {
        /// Collects all tags in a vector
        fn collect_tags(groups: &Vec<Group>) -> Vec<Tag> {
            let mut tags: Vec<Tag> = vec![];
            for group in groups {
                let mut group_tags = group.tags.to_vec();
                tags.append(&mut group_tags);
            }

            tags
        }

        let tags: Vec<Tag> = collect_tags(groups);

        TagValidator {
            groups: groups.to_vec(),
            tags,
        }
    }

    /// Checks for any groups that aren't the default and checks for missing groups that are default
    pub fn validate_groups(&self) -> bool {
        if self.groups.len() < 3 {
            println!("There are less than three groups! Make sure that all groups are in tags.txt!");
            sleep(Duration::from_millis(1000));
            return false;
        }

        for group in &self.groups {
            match group.group_name.as_str() {
                "artists" => continue,
                "normal-tags" => continue,
                "pools" => continue,
                _ => {
                    println!("Group {} is unknown!", group.group_name);
                    sleep(Duration::from_millis(1000));
                    return false;
                },
            }
        }

        true
    }
}
