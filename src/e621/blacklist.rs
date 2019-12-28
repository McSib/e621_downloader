use crate::e621::caller::PostEntry;
use failure::Error;

#[derive(Debug)]
struct BlacklistLineToken {
    tags: Vec<BlacklistTagToken>,
}

impl BlacklistLineToken {
    fn new(tags: Vec<BlacklistTagToken>) -> Self {
        let mut line_token = BlacklistLineToken::default();
        line_token.tags = tags;
        line_token
    }
}

impl Default for BlacklistLineToken {
    fn default() -> Self {
        BlacklistLineToken { tags: Vec::new() }
    }
}

#[derive(Debug, PartialEq)]
enum BlacklistRating {
    None,
    Safe,
    Questionable,
    Explicit,
}

#[derive(Debug, PartialEq)]
enum BlacklistID {
    None,
    ID(i64),
}

#[derive(Debug, PartialEq)]
enum BlacklistUser {
    None,
    User(String),
}

#[derive(Debug)]
struct BlacklistTagToken {
    negated: bool,
    rating: BlacklistRating,
    id: BlacklistID,
    user: BlacklistUser,
    name: String,
}

impl BlacklistTagToken {
    fn new(name: &str) -> Self {
        BlacklistTagToken {
            negated: false,
            rating: BlacklistRating::None,
            id: BlacklistID::None,
            user: BlacklistUser::None,
            name: name.to_string(),
        }
    }

    fn is_user(&self) -> bool {
        self.user != BlacklistUser::None
    }

    fn is_id(&self) -> bool {
        self.id != BlacklistID::None
    }

    fn is_rating(&self) -> bool {
        self.rating != BlacklistRating::None
    }
}

impl Default for BlacklistTagToken {
    fn default() -> Self {
        BlacklistTagToken {
            negated: false,
            rating: BlacklistRating::None,
            id: BlacklistID::None,
            user: BlacklistUser::None,
            name: String::new(),
        }
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
    fn parse_tags(&mut self) -> Result<BlacklistLineToken, Error> {
        let mut tags: Vec<BlacklistTagToken> = Vec::new();
        loop {
            self.consume_whitespace();
            if self.eof() {
                break;
            }

            tags.push(self.parse_tag()?);
        }

        Ok(BlacklistLineToken::new(tags))
    }

    fn parse_tag(&mut self) -> Result<BlacklistTagToken, Error> {
        if self.starts_with("rating:") || self.starts_with("id:") || self.starts_with("user:") {
            let mut token = BlacklistTagToken::default();
            let identifier = self.consume_while(valid_tag);

            assert_eq!(self.consume_char(), ':');

            let value = self.process_value(&identifier)?;
            self.update_token(&mut token, &identifier, &value)?;
            return Ok(token);
        }

        if self.starts_with("-") {
            assert_eq!(self.consume_char(), '-');

            let name = self.consume_while(valid_tag);
            let mut token = BlacklistTagToken::new(&name);
            token.negated = true;
            return Ok(token);
        }

        let name = self.consume_while(valid_tag);
        Ok(BlacklistTagToken::new(&name))
    }

    fn process_value(&mut self, identifier: &str) -> Result<String, Error> {
        match identifier {
            "rating" => Ok(self.consume_while(valid_rating)),
            "id" => Ok(self.consume_while(valid_id)),
            "user" => Ok(self.consume_while(valid_user)),
            _ => bail!(format_err!(
                "Identifier doesn't match with any match parameters!"
            )),
        }
    }

    fn update_token(
        &self,
        token: &mut BlacklistTagToken,
        identifier: &str,
        value: &str,
    ) -> Result<(), Error> {
        token.name = identifier.to_string();
        match identifier {
            "rating" => {
                token.rating = self.get_rating(&value);
            }
            "id" => {
                token.id = BlacklistID::ID(value.parse::<i64>()?);
            }
            "user" => {
                token.user = BlacklistUser::User(value.to_string());
            }
            _ => bail!(format_err!(
                "Identifier doesn't match with any match parameters!"
            )),
        };

        Ok(())
    }

    fn get_rating(&self, value: &str) -> BlacklistRating {
        match value.to_lowercase().as_str() {
            "safe" | "s" => BlacklistRating::Safe,
            "questionable" | "q" => BlacklistRating::Questionable,
            "explicit" | "e" => BlacklistRating::Explicit,
            _ => BlacklistRating::None,
        }
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

    fn set_red_flag_margin(&mut self, tags: &[BlacklistTagToken]) {
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

    fn check_post(&mut self, post: &PostEntry, blacklist_line: &BlacklistLineToken) {
        let mut flags: i16 = 0;
        let mut negated_flags: i16 = 0;
        let post_tags: Vec<&str> = post.tags.split(' ').collect();
        for tag in &blacklist_line.tags {
            if tag.is_user() || tag.is_rating() || tag.is_id() {
                if tag.is_id() {
                    if let BlacklistID::ID(id) = tag.id {
                        if post.id == id {
                            self.flagged = true;
                            break;
                        }
                    }
                }

                if tag.is_user() {
                    if let BlacklistUser::User(username) = &tag.user {
                        if post.author == *username {
                            self.flagged = true;
                            break;
                        }
                    }
                }

                if tag.is_rating() {
                    match tag.rating {
                        BlacklistRating::None => {}
                        BlacklistRating::Safe => {
                            if post.rating.as_str() == "s" {
                                flags += 1;
                                continue;
                            }
                        }
                        BlacklistRating::Questionable => {
                            if post.rating.as_str() == "q" {
                                flags += 1;
                                continue;
                            }
                        }
                        BlacklistRating::Explicit => {
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
            let mut parser = Parser {
                pos: 0,
                input: blacklist_entry.to_string(),
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
