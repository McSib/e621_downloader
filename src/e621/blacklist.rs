extern crate failure;

use failure::Error;

use crate::e621::io::parser::Parser;
use crate::e621::sender::{PostEntry, RequestSender, UserEntry};
use reqwest::get;

#[derive(Default, Debug)]
struct RootToken {
    lines: Vec<LineToken>,
}

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
        self.user.is_some()
    }

    fn is_id(&self) -> bool {
        self.id.is_some()
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
// TODO: Remove default when new blacklist is made.
#[derive(Default)]
struct BlacklistParser {
    base_parser: Parser,
}

impl BlacklistParser {
    fn new(blacklist: String) -> Self {
        BlacklistParser {
            base_parser: Parser {
                pos: 0,
                input: blacklist,
            },
        }
    }

    fn parse_blacklist(&mut self) -> RootToken {
        let mut lines: Vec<LineToken> = Vec::new();
        loop {
            self.base_parser.consume_whitespace();
            if self.base_parser.eof() {
                break;
            }

            lines.push(self.parse_line());
        }

        RootToken { lines }
    }

    /// Parses each tag and collects them before return a [`LineToken`].
    fn parse_line(&mut self) -> LineToken {
        let mut tags: Vec<TagToken> = Vec::new();
        loop {
            if self.base_parser.starts_with("\n") {
                assert_eq!(self.base_parser.consume_char(), '\n');
                break;
            }

            self.base_parser.consume_whitespace();
            if self.base_parser.eof() {
                break;
            }

            tags.push(self.parse_tag());
        }

        LineToken::new(tags)
    }

    /// Checks if tag starts with any special syntax.
    fn is_tag_special(&self, tag: &String) -> bool {
        tag == "rating" || tag == "id" || tag == "user"
    }

    /// Checks if tag is negated.
    fn is_tag_negated(&self) -> bool {
        self.base_parser.starts_with("-")
    }

    /// Parses tag and runs through basic identification before returning it as a [`TagToken`].
    fn parse_tag(&mut self) -> TagToken {
        let mut token = TagToken::default();
        if self.is_tag_negated() {
            assert_eq!(self.base_parser.consume_char(), '-');
            token.negated = true;
        }

        token.tag = self.base_parser.consume_while(valid_tag).to_lowercase();

        // This will be considered a special tag if it contains the syntax of one.
        if !self.base_parser.eof() && self.base_parser.next_char() == ':' {
            self.parse_special_tag(&mut token);
        }

        println!("{:#?}", token);

        token
    }

    /// Parses special tag and updates token with the appropriate type and value.
    ///
    /// # Panic
    /// If identifier doesn't match with any of the match arms, it will fail and throw an `Error`.
    fn parse_special_tag(&mut self, token: &mut TagToken) {
        assert_eq!(self.base_parser.consume_char(), ':');
        match token.tag.as_str() {
            "rating" => {
                let rating_string = self.base_parser.consume_while(valid_rating);
                token.rating = self.get_rating(&rating_string);
            }
            "id" => {
                token.id = Some(
                    self.base_parser
                        .consume_while(valid_id)
                        .parse::<i64>()
                        .unwrap_or_default(),
                );
            }
            "user" => {
                token.user = Some(self.base_parser.consume_while(valid_user));
            }
            _ => {}
        };
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

/// The flag worker flags and removes any grabbed post that matches with all of the tags in a `LineToken`.
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
    /// Resets the flag worker for the next post.
    fn reset_worker(&mut self) {
        *self = Self::default();
    }

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

        // println!("Margin: {}", length);
        // println!("Negated Margin: {}", negated_length);

        self.margin = length;
        self.negated_margin = negated_length;
    }

    /// Flags post based on blacklisted rating.
    fn flag_rating(
        &self,
        flags: &mut i16,
        negated_flags: &mut i16,
        tag: &TagToken,
        post: &PostEntry,
    ) {
        match tag.rating {
            Rating::Safe => {
                if post.rating == "s" {
                    if tag.negated {
                        *negated_flags += 1;
                    // println!(
                    //     "Negated flag raised {}/{}",
                    //     negated_flags, self.negated_margin
                    // );
                    } else {
                        *flags += 1;
                        // println!("Flag raised {}/{}", flags, self.margin);
                    }
                }
            }
            Rating::Questionable => {
                if post.rating == "q" {
                    if tag.negated {
                        *negated_flags += 1;
                    // println!(
                    //     "Negated flag raised {}/{}",
                    //     negated_flags, self.negated_margin
                    // );
                    } else {
                        *flags += 1;
                        // println!("Flag raised {}/{}", flags, self.margin);
                    }
                }
            }
            Rating::Explicit => {
                if post.rating == "e" {
                    if tag.negated {
                        *negated_flags += 1;
                    // println!(
                    //     "Negated flag raised {}/{}",
                    //     negated_flags, self.negated_margin
                    // );
                    } else {
                        *flags += 1;
                        // println!("Flag raised {}/{}", flags, self.margin);
                    }
                }
            }
            Rating::None => unreachable!(),
        }
    }

    /// Raises the flag and immediately blacklists the post if its ID matches with the blacklisted ID.
    fn flag_id(&mut self, flags: &mut i16, negated_flags: &mut i16, tag: &TagToken, post_id: i64) {
        if tag.is_id() {
            if let Some(id) = tag.id {
                if post_id == id {
                    if tag.negated {
                        *negated_flags += 1;
                    // println!(
                    //     "Negated flag raised {}/{}",
                    //     negated_flags, self.negated_margin
                    // );
                    } else {
                        *flags += 1;
                        // println!("Flag raised {}/{}", flags, self.margin);
                    }
                }
            }
        }
    }

    /// Raises the flag and immediately blacklists the post if the user who uploaded it is blacklisted.
    fn flag_user(&mut self, flags: &mut i16, negated_flags: &mut i16, tag: &TagToken) {
        if tag.is_user() {
            if let Some(username) = &tag.user {
                if tag.tag == *username {
                    if tag.negated {
                        *negated_flags += 1;
                    // println!(
                    //     "Negated flag raised {}/{}",
                    //     negated_flags, self.negated_margin
                    // );
                    } else {
                        *flags += 1;
                        // println!("Flag raised {}/{}", flags, self.margin);
                    }
                }
            }
        }
    }

    /// Checks if a single post is blacklisted or safe.
    fn check_post(&mut self, post: &PostEntry, blacklist_line: &LineToken) {
        let mut flags: i16 = 0;
        let mut negated_flags: i16 = 0;
        let post_tags = post.tags.clone().combine_tags();
        for tag in &blacklist_line.tags {
            if tag.is_special() {
                if tag.is_id() {
                    self.flag_id(&mut flags, &mut negated_flags, tag, post.id);
                    continue;
                }

                if tag.is_user() {
                    self.flag_user(&mut flags, &mut negated_flags, tag);
                    continue;
                }

                if tag.is_rating() {
                    self.flag_rating(&mut flags, &mut negated_flags, tag, post);
                    continue;
                }
            } else if post_tags.iter().any(|e| e == tag.tag.as_str()) {
                if tag.is_negated() {
                    negated_flags += 1;
                // println!(
                //     "Negated flag raised {}/{}",
                //     negated_flags, self.negated_margin
                // );
                } else {
                    flags += 1;
                    // println!("Flag raised {}/{}", flags, self.margin);
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
    blacklist_parser: BlacklistParser,
    blacklist_tokens: RootToken,
    request_sender: RequestSender,
    blacklist_entries: Vec<String>,
}

impl Blacklist {
    pub fn new(request_sender: RequestSender) -> Self {
        Blacklist {
            blacklist_parser: BlacklistParser::default(),
            blacklist_tokens: RootToken::default(),
            request_sender,
            blacklist_entries: Vec::default(),
        }
    }

    pub fn parse_blacklist(&mut self, user_blacklist: String) {
        self.blacklist_parser = BlacklistParser::new(user_blacklist);
        self.blacklist_tokens = self.blacklist_parser.parse_blacklist();
    }

    pub fn cache_users(&mut self) {
        for blacklist_token in &mut self.blacklist_tokens.lines {
            for tag in &mut blacklist_token.tags {
                if tag.is_user() {
                    let user: UserEntry = self
                        .request_sender
                        .get_entry_from_appended_id(&tag.tag, "user");
                    tag.tag = user.name;
                }
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blacklist_tokens.lines.is_empty()
    }

    /// Filters through a set of posts, only retaining posts that aren't blacklisted.
    pub fn filter_posts(&self, posts: &mut Vec<PostEntry>) {
        let mut filtered: u16 = 0;
        let mut flag_worker = FlagWorker::default();
        // println!("{:#?}", self.blacklist_tokens);
        for blacklist_line in &self.blacklist_tokens.lines {
            posts.retain(|e| {
                flag_worker.reset_worker();
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
