use failure::ResultExt;

use crate::e621::io::parser::BaseParser;
use crate::e621::sender::entries::{PostEntry, UserEntry};
use crate::e621::sender::RequestSender;

/// Root token which contains all the tokens of the blacklist.
#[derive(Default, Debug)]
struct RootToken {
    lines: Vec<LineToken>,
}

/// Parsed line token that contains all collected [`TagToken`]s on single input line.
#[derive(Debug, Default)]
struct LineToken {
    tags: Vec<TagToken>,
}

impl LineToken {
    fn new(tags: Vec<TagToken>) -> Self {
        LineToken { tags }
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

#[derive(Debug)]
enum TagType {
    Rating(Rating),
    Id(Option<i64>),
    User(Option<String>),
    None,
}

/// Tag token that contains essential information about what is blacklisted.
/// Whether the post is a rating, ID, user, or just a plain tag, this token keeps all the information and ensures that you know everything about this tag.
#[derive(Debug)]
struct TagToken {
    /// If the tag is negated or not
    negated: bool,
    /// If the tag is a rating, this will hold the exact the rating it is
    tag_type: TagType,
    /// The tag (value for special tags)
    name: String,
}

impl Default for TagToken {
    fn default() -> Self {
        TagToken {
            negated: false,
            tag_type: TagType::None,
            name: String::new(),
        }
    }
}

/// Parser that reads a tag file and parses the tags.
#[derive(Default)]
struct BlacklistParser {
    /// The base parser which parses the blacklist character by character.
    base_parser: BaseParser,
}

impl BlacklistParser {
    fn new(blacklist: String) -> Self {
        trace!("Initializing blacklist parser...");
        BlacklistParser {
            base_parser: BaseParser::new(blacklist),
        }
    }

    /// Parses the entire blacklist.
    fn parse_blacklist(&mut self) -> RootToken {
        trace!("Parsing blacklist...");
        let mut lines: Vec<LineToken> = Vec::new();
        loop {
            self.base_parser.consume_whitespace();
            if self.base_parser.eof() {
                break;
            }

            lines.push(self.parse_line());
        }

        trace!("Parsed blacklist...");

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

        token.name = self.base_parser.consume_while(valid_tag).to_lowercase();

        // This will be considered a special tag if it contains the syntax of one.
        if !self.base_parser.eof() && self.base_parser.next_char() == ':' {
            self.parse_special_tag(&mut token);
        }

        token
    }

    /// Parses special tag and updates token with the appropriate type and value.
    ///
    /// # Panic
    /// If identifier doesn't match with any of the match arms, it will fail and throw an `Error`.
    fn parse_special_tag(&mut self, token: &mut TagToken) {
        assert_eq!(self.base_parser.consume_char(), ':');
        match token.name.as_str() {
            "rating" => {
                let rating_string = self.base_parser.consume_while(valid_rating);
                token.tag_type = TagType::Rating(self.get_rating(&rating_string));
            }
            "id" => {
                token.tag_type = TagType::Id(Some(
                    self.base_parser
                        .consume_while(valid_id)
                        .parse::<i64>()
                        .unwrap_or_default(),
                ));
            }
            "user" => {
                token.tag_type = TagType::User(Some(self.base_parser.consume_while(valid_user)));
            }
            _ => {
                self.base_parser.report_error(
                    format!("Unknown special tag identifier: {}", token.name).as_str(),
                );
            }
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
    matches!(c, 'A'..='Z' | 'a'..='z')
}

/// Validates character for id.
fn valid_id(c: char) -> bool {
    matches!(c, '0'..='9')
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
    /// The number of flags raised by the worker
    flags: i16,
    /// The number of negated flags raised by the worker
    negated_flags: i16,
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
        tags.iter().for_each(|e| {
            if e.negated {
                self.negated_margin += 1;
            } else {
                self.margin += 1;
            }
        });
    }

    /// Flags post based on blacklisted rating.
    fn flag_rating(&mut self, rating: &Rating, post: &PostEntry, negated: bool) {
        match (rating, post.rating.as_str()) {
            (Rating::Safe, "s") | (Rating::Questionable, "q") | (Rating::Explicit, "e") => {
                self.raise_flag(negated);
            }
            (_, _) => {}
        }
    }

    /// Raises the flag and immediately blacklists the post if its ID matches with the blacklisted ID.
    fn flag_id(&mut self, id: i64, post_id: i64, negated: bool) {
        if post_id == id {
            self.raise_flag(negated);
        }
    }

    /// Raises the flag and immediately blacklists the post if the user who uploaded it is blacklisted.
    fn flag_user(&mut self, user_id: i64, uploader_id: i64, negated: bool) {
        if user_id == uploader_id {
            self.raise_flag(negated);
        }
    }

    /// Checks if a single post is blacklisted or safe.
    fn check_post(&mut self, post: &PostEntry, blacklist_line: &LineToken) {
        let post_tags = post.tags.clone().combine_tags();
        for tag in &blacklist_line.tags {
            match &tag.tag_type {
                TagType::Rating(rating) => {
                    self.flag_rating(rating, post, tag.negated);
                    continue;
                }
                TagType::Id(id) => {
                    if let Some(blacklisted_id) = id {
                        self.flag_id(*blacklisted_id, post.id, tag.negated);
                    }
                    continue;
                }
                TagType::User(_) => {
                    let user_id = tag
                        .name
                        .parse::<i64>()
                        .with_context(|e| {
                            error!("Failed to parse blacklisted user id: {}!", tag.name);
                            format!("{}", e)
                        })
                        .unwrap();
                    self.flag_user(user_id, post.uploader_id, tag.negated);
                    continue;
                }
                TagType::None => {
                    if post_tags.iter().any(|e| e == tag.name.as_str()) {
                        self.raise_flag(tag.negated);
                    }
                }
            }
        }

        if self.negated_margin != 0 && self.negated_flags == self.negated_margin {
            self.flagged = false;
        } else if self.flags == self.margin {
            self.flagged = true;
        }
    }

    fn raise_flag(&mut self, negated: bool) {
        if negated {
            self.negated_flags += 1;
        } else {
            self.flags += 1;
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
    /// The blacklist parser which parses the blacklist and tokenizes it.
    blacklist_parser: BlacklistParser,
    /// All of the blacklist tokens after being parsed.
    blacklist_tokens: RootToken,
    /// Request sender used for getting user information.
    request_sender: RequestSender,
}

impl Blacklist {
    pub fn new(request_sender: RequestSender) -> Self {
        Blacklist {
            blacklist_parser: BlacklistParser::default(),
            blacklist_tokens: RootToken::default(),
            request_sender,
        }
    }

    pub fn parse_blacklist(&mut self, user_blacklist: String) -> &mut Blacklist {
        self.blacklist_parser = BlacklistParser::new(user_blacklist);
        self.blacklist_tokens = self.blacklist_parser.parse_blacklist();
        self
    }

    /// Goes through all of the blacklisted users and obtains there ID for the flagging system to cross examine with posts.
    pub fn cache_users(&mut self) {
        for blacklist_token in &mut self.blacklist_tokens.lines {
            for tag in &mut blacklist_token.tags {
                if let TagType::User(Some(username)) = &tag.tag_type {
                    let user: UserEntry = self
                        .request_sender
                        .get_entry_from_appended_id(username, "user");
                    tag.name = format!("{}", user.id);
                }
            }
        }
    }

    /// Checks of the blacklist is empty.
    pub fn is_empty(&self) -> bool {
        self.blacklist_tokens.lines.is_empty()
    }

    /// Filters through a set of posts, only retaining posts that aren't blacklisted.
    ///
    /// # Arguments
    ///
    /// * `posts`: Posts to filter through.
    ///
    /// returns: u16 of posts that were positively filtered
    pub fn filter_posts(&self, posts: &mut Vec<PostEntry>) -> u16 {
        let mut filtered: u16 = 0;
        let mut flag_worker = FlagWorker::default();
        for blacklist_line in &self.blacklist_tokens.lines {
            posts.retain(|e| {
                flag_worker.reset_worker();
                flag_worker.set_flag_margin(&blacklist_line.tags);
                flag_worker.check_post(e, blacklist_line);
                if flag_worker.is_flagged() {
                    filtered += 1;
                }

                // This inverses the flag to make sure it retains what isn't flagged and disposes of
                // what is flagged.
                !flag_worker.is_flagged()
            });
        }

        if filtered > 0 {
            trace!("Filtered {} posts with blacklist...", filtered);
        } else {
            trace!("No posts filtered...");
        }

        filtered
    }
}
