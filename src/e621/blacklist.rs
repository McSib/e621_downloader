use crate::e621::io::parser::Parser;
use crate::e621::sender::PostEntry;
use failure::Error;

/// Parsed line token that contains all collected [`TagToken`]s on single input line.
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

/// Enum that contains each possible option from `rating:` being in blacklist.
#[derive(Debug, PartialEq)]
enum Rating {
    None,
    Safe,
    Questionable,
    Explicit,
}

/// Tag token that contains essential information about what is blacklisted.
/// Whether the post is a rating, ID, user, or just a plain tag, this token keeps all the information and ensures that you know everything about this tag.
#[derive(Debug)]
struct TagToken {
    /// If the tag is negated or not
    negated: bool,
    /// If the tag is a rating, this will hold the exact the rating it is
    rating: Rating,
    /// If the tag is an ID, this will hold the ID number
    id: Option<i64>,
    /// If the tag is a user, this will contain the username
    user: Option<String>,
    tag: String,
}

impl TagToken {
    fn is_user(&self) -> bool {
        self.user.is_none()
    }

    fn is_id(&self) -> bool {
        self.id.is_none()
    }

    fn is_rating(&self) -> bool {
        self.rating != Rating::None
    }

    fn is_special(&self) -> bool {
        self.is_user() || self.is_rating() || self.is_id()
    }

    fn is_negated(&self) -> bool {
        self.negated
    }
}

impl Default for TagToken {
    fn default() -> Self {
        TagToken {
            negated: false,
            rating: Rating::None,
            id: None,
            user: None,
            tag: String::new(),
        }
    }
}

/// Parser that reads a tag file and parses the tags.
struct BlacklistParser {
    parser: Parser,
}

impl BlacklistParser {
    /// Parses each tag and collects them before return a [`LineToken`].
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

    /// Checks if tag starts with any special syntax.
    fn is_tag_special(&self) -> bool {
        self.parser.starts_with("rating:")
            || self.parser.starts_with("id:")
            || self.parser.starts_with("user:")
    }

    /// Checks if tag is negated.
    fn is_tag_negated(&self) -> bool {
        self.parser.starts_with("-")
    }

    /// Parses tag and runs through basic identification before returning it as a [`TagToken`].
    fn parse_tag(&mut self) -> Result<TagToken, Error> {
        let mut token = TagToken::default();
        if self.is_tag_negated() {
            assert_eq!(self.parser.consume_char(), '-');

            token.tag = self.parser.consume_while(valid_tag);
            token.negated = true;
        } else {
            token.tag = self.parser.consume_while(valid_tag);
        }

        if self.is_tag_special() {
            self.parse_special_tag(&mut token)?;
        }

        Ok(token)
    }

    /// Parses special tag and updates token with the appropriate type and value.
    fn parse_special_tag(&mut self, token: &mut TagToken) -> Result<(), Error> {
        assert_eq!(self.parser.consume_char(), ':');
        let value = self.parse_value(&token.tag)?;
        match token.tag.as_str() {
            "rating" => {
                token.rating = self.get_rating(&value);
            }
            "id" => {
                token.id = Some(value.parse::<i64>()?);
            }
            "user" => {
                token.user = Some(value);
            }
            _ => bail!(format_err!(
                "Identifier doesn't match with any match parameters!"
            )),
        };

        Ok(())
    }

    /// Parses value and returns it.
    fn parse_value(&mut self, identifier: &str) -> Result<String, Error> {
        match identifier {
            "rating" => Ok(self.parser.consume_while(valid_rating)),
            "id" => Ok(self.parser.consume_while(valid_id)),
            "user" => Ok(self.parser.consume_while(valid_user)),
            _ => bail!(format_err!(
                "Identifier doesn't match with any match parameters!"
            )),
        }
    }

    /// Checks parsed value and returns the correct rating for it.
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

/// The flag worker flags and removes any grabbed post that matches with all of the tags in a [`LineToken`].
/// The worker works with the supplied syntax and rules on e621's main site listed [here](https://e621.net/help/show/blacklist).
/// This ensures that the client-side blacklist works exactly the same as the server-side blacklist.
///
/// # Important
/// When doing any modifications to the worker, be sure to test the blacklist on client and server-side.
/// This will ensure that there aren't any unexpected behavior, or issues with the worker that weren't noticed.
/// A good thing to focus on is how many posts are blacklisted in total.
/// If the site says 236 posts are blacklisted, and the program is saying only 195 are blacklisted, it's safe to assume there is a problem with how the worker is blacklisting posts.
#[derive(Default)]
struct FlagWorker {
    /// The margin of how many flags that should be raised before a post is determined to be blacklisted
    margin: i16,
    /// The margin of how many negated flags that should be raised before a post is determined to be safe
    negated_margin: i16,
    /// Whether the post is flagged or not
    flagged: bool,
}

impl FlagWorker {
    /// Sets margin for how many flags need to be raised before the post is either blacklisted or considered safe.
    fn set_flag_margin(&mut self, tags: &[TagToken]) {
        let mut negated_length: i16 = 0;
        let mut length: i16 = 0;
        tags.iter().for_each(|e| {
            if e.is_negated() {
                negated_length += 1;
            } else {
                length += 1;
            }
        });

        self.margin = length;
        self.negated_margin = negated_length;
    }

    /// Flags post based on blacklisted rating.
    fn flag_rating(&self, flags: &mut i16, tag: &TagToken, post: &PostEntry) {
        match tag.rating {
            Rating::Safe => {
                if post.rating == "s" {
                    *flags += 1;
                }
            }
            Rating::Questionable => {
                if post.rating == "q" {
                    *flags += 1;
                }
            }
            Rating::Explicit => {
                if post.rating == "e" {
                    *flags += 1;
                }
            }
            Rating::None => unreachable!()
        }
    }
    
    /// Raises the flag and immediately blacklists the post if its ID matches with the blacklisted ID.
    fn flag_id(&mut self, tag: &TagToken, post: &PostEntry) -> bool {
        if tag.is_id() {
            if let Some(id) = tag.id {
                if post.id == id {
                    self.flagged = true;
                    return true;
                }
            }
        }
        
        false
    }

    /// Raises the flag and immediately blacklists the post if the user who uploaded it is blacklisted.
    fn flag_user(&mut self, tag: &TagToken, post: &PostEntry) -> bool {
        if tag.is_user() {
            if let Some(username) = &tag.user {
                if post.author == *username {
                    self.flagged = true;
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Checks if a single post is blacklisted or safe.
    fn check_post(&mut self, post: &PostEntry, blacklist_line: &LineToken) {
        let mut flags: i16 = 0;
        let mut negated_flags: i16 = 0;
        let post_tags: Vec<&str> = post.tags.split(' ').collect();
        for tag in &blacklist_line.tags {
            if tag.is_special() {
                if self.flag_id(tag, post) {
                    break;
                }

                if self.flag_user(tag, post) {
                    break;
                }

                if tag.is_rating() {
                    self.flag_rating(&mut flags, tag, post);
                    continue;
                }
            } else {
                for post_tag in &post_tags {
                    if *post_tag == tag.tag {
                        if tag.is_negated() {
                            negated_flags += 1;
                        } else {
                            flags += 1;
                        }
                    }
                }

                for artist in &post.artist {
                    if *artist == tag.tag {
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

    /// Returns if the flag is raised or not.
    fn is_flagged(&self) -> bool {
        self.flagged
    }
}

/// Blacklist that holds all of the blacklist entries.
/// These entries will be looped through a parsed before being used for filtering posts that are blacklisted. 
pub struct Blacklist {
    blacklist_entries: Vec<String>,
}

impl Blacklist {
    pub fn new(blacklist_entries: &[String]) -> Self {
        Blacklist {
            blacklist_entries: blacklist_entries.to_vec(),
        }
    }

    /// Filters through a set of posts, only retaining posts that aren't blacklisted.
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
                let mut flag_worker = FlagWorker::default();
                flag_worker.set_flag_margin(&blacklist_line.tags);
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
