use crate::e621::io::parser::Parser;
use crate::e621::sender::PostEntry;
use failure::Error;

#[derive(Debug)]
struct LineToken {
    tags: Vec<TagToken>,
}

impl LineToken {
    fn new(tags: Vec<TagToken>) -> Self {
        let mut line_token = LineToken::default();
        line_token.tags = tags;
        line_token
    }
}

impl Default for LineToken {
    fn default() -> Self {
        LineToken { tags: Vec::new() }
    }
}

#[derive(Debug, PartialEq)]
enum Rating {
    None,
    Safe,
    Questionable,
    Explicit,
}

#[derive(Debug, PartialEq)]
enum ID {
    None,
    ID(i64),
}

#[derive(Debug, PartialEq)]
enum User {
    None,
    User(String),
}

#[derive(Debug)]
struct TagToken {
    negated: bool,
    rating: Rating,
    id: ID,
    user: User,
    name: String,
}

impl TagToken {
    fn new(name: &str) -> Self {
        TagToken {
            negated: false,
            rating: Rating::None,
            id: ID::None,
            user: User::None,
            name: name.to_string(),
        }
    }

    fn is_user(&self) -> bool {
        self.user != User::None
    }

    fn is_id(&self) -> bool {
        self.id != ID::None
    }

    fn is_rating(&self) -> bool {
        self.rating != Rating::None
    }
}

impl Default for TagToken {
    fn default() -> Self {
        TagToken {
            negated: false,
            rating: Rating::None,
            id: ID::None,
            user: User::None,
            name: String::new(),
        }
    }
}

/// Parser that reads a tag file and parses the tags.
struct BlacklistParser {
    parser: Parser,
}

impl BlacklistParser {
    fn parse_tags(&mut self) -> Result<LineToken, Error> {
        let mut tags: Vec<TagToken> = Vec::new();
        loop {
            self.parser.consume_whitespace();
            if self.parser.eof() {
                break;
            }

            tags.push(self.parse_tag()?);
        }

        Ok(LineToken::new(tags))
    }

    fn parse_tag(&mut self) -> Result<TagToken, Error> {
        if self.parser.starts_with("rating:")
            || self.parser.starts_with("id:")
            || self.parser.starts_with("user:")
        {
            let mut token = TagToken::default();
            let identifier = self.parser.consume_while(valid_tag);

            assert_eq!(self.parser.consume_char(), ':');

            let value = self.process_value(&identifier)?;
            self.update_token(&mut token, &identifier, &value)?;
            return Ok(token);
        }

        if self.parser.starts_with("-") {
            assert_eq!(self.parser.consume_char(), '-');

            let name = self.parser.consume_while(valid_tag);
            let mut token = TagToken::new(&name);
            token.negated = true;
            return Ok(token);
        }

        let name = self.parser.consume_while(valid_tag);
        Ok(TagToken::new(&name))
    }

    fn process_value(&mut self, identifier: &str) -> Result<String, Error> {
        match identifier {
            "rating" => Ok(self.parser.consume_while(valid_rating)),
            "id" => Ok(self.parser.consume_while(valid_id)),
            "user" => Ok(self.parser.consume_while(valid_user)),
            _ => bail!(format_err!(
                "Identifier doesn't match with any match parameters!"
            )),
        }
    }

    fn update_token(
        &self,
        token: &mut TagToken,
        identifier: &str,
        value: &str,
    ) -> Result<(), Error> {
        token.name = identifier.to_string();
        match identifier {
            "rating" => {
                token.rating = self.get_rating(&value);
            }
            "id" => {
                token.id = ID::ID(value.parse::<i64>()?);
            }
            "user" => {
                token.user = User::User(value.to_string());
            }
            _ => bail!(format_err!(
                "Identifier doesn't match with any match parameters!"
            )),
        };

        Ok(())
    }

    fn get_rating(&self, value: &str) -> Rating {
        match value.to_lowercase().as_str() {
            "safe" | "s" => Rating::Safe,
            "questionable" | "q" => Rating::Questionable,
            "explicit" | "e" => Rating::Explicit,
            _ => Rating::None,
        }
    }
}

/// Validates character for tag.
fn valid_tag(c: char) -> bool {
    match c {
        '!'..='9' | ';'..='~' => true,
        // This will check for any special characters in the validator.
        _ => {
            if c != ':' {
                return c.is_alphanumeric();
            }

            false
        }
    }
}

/// Validates character for user.
fn valid_user(c: char) -> bool {
    match c {
        '!'..='9' | ';'..='~' => true,
        // This will check for any special characters in the validator.
        _ => {
            if c != ':' {
                return c.is_alphanumeric();
            }

            false
        }
    }
}

/// Validates character for rating.
fn valid_rating(c: char) -> bool {
    match c {
        'A'..='Z' => true,
        'a'..='z' => true,
        _ => false,
    }
}

/// Validates character for id.
fn valid_id(c: char) -> bool {
    match c {
        '0'..='9' => true,
        _ => false,
    }
}

struct BlacklistFlagWorker {
    margin: i16,
    negated_margin: i16,
    flagged: bool,
}

impl BlacklistFlagWorker {
    fn new() -> Self {
        BlacklistFlagWorker {
            margin: 0,
            negated_margin: 0,
            flagged: false,
        }
    }

    fn set_red_flag_margin(&mut self, tags: &[TagToken]) {
        let (length, negated_length) = {
            let mut negated_length: i16 = 0;
            let mut length: i16 = tags.len() as i16;

            for tag in tags {
                if tag.negated {
                    negated_length += 1;
                    length -= 1;
                }
            }

            (length, negated_length)
        };
        self.margin = length;
        self.negated_margin = negated_length;
    }

    fn check_post(&mut self, post: &PostEntry, blacklist_line: &LineToken) {
        let mut flags: i16 = 0;
        let mut negated_flags: i16 = 0;
        let post_tags: Vec<&str> = post.tags.split(' ').collect();
        for tag in &blacklist_line.tags {
            if tag.is_user() || tag.is_rating() || tag.is_id() {
                if tag.is_id() {
                    if let ID::ID(id) = tag.id {
                        if post.id == id {
                            self.flagged = true;
                            break;
                        }
                    }
                }

                if tag.is_user() {
                    if let User::User(username) = &tag.user {
                        if post.author == *username {
                            self.flagged = true;
                            break;
                        }
                    }
                }

                if tag.is_rating() {
                    match tag.rating {
                        Rating::None => {}
                        Rating::Safe => {
                            if post.rating.as_str() == "s" {
                                flags += 1;
                                continue;
                            }
                        }
                        Rating::Questionable => {
                            if post.rating.as_str() == "q" {
                                flags += 1;
                                continue;
                            }
                        }
                        Rating::Explicit => {
                            if post.rating.as_str() == "e" {
                                flags += 1;
                                continue;
                            }
                        }
                    }
                }
            } else {
                for post_tag in &post_tags {
                    if *post_tag == tag.name {
                        if tag.negated {
                            negated_flags += 1;
                        } else {
                            flags += 1;
                        }
                    }
                }

                for artist in &post.artist {
                    if *artist == tag.name {
                        flags += 1;
                        break;
                    }
                }
            }
        }

        if self.negated_margin != 0 && negated_flags == self.negated_margin {
            self.flagged = false;
        } else if flags == self.margin {
            self.flagged = true;
        }
    }

    fn is_flagged(&self) -> bool {
        self.flagged
    }
}

pub struct Blacklist {
    blacklist_entries: Vec<String>,
}

impl Blacklist {
    pub fn new(blacklist_entries: &[String]) -> Self {
        Blacklist {
            blacklist_entries: blacklist_entries.to_vec(),
        }
    }

    pub fn filter_posts(&self, posts: &mut Vec<PostEntry>) {
        let mut filtered = 0;
        for blacklist_entry in &self.blacklist_entries {
            let mut parser = BlacklistParser {
                parser: Parser {
                    pos: 0,
                    input: blacklist_entry.to_string(),
                },
            };
            let blacklist_line = parser.parse_tags().unwrap();
            posts.retain(|e| {
                let mut flag_worker = BlacklistFlagWorker::new();
                flag_worker.set_red_flag_margin(&blacklist_line.tags);
                flag_worker.check_post(e, &blacklist_line);
                if flag_worker.is_flagged() {
                    filtered += 1;
                }

                !flag_worker.is_flagged()
            });
        }

        println!("Filtered {} posts with blacklist...", filtered)
    }
}
