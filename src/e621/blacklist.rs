/*
 * Copyright (c) 2022 McSib
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use failure::ResultExt;
use std::cmp::Ordering;

use crate::e621::{
    io::parser::BaseParser,
    sender::{
        entries::{PostEntry, UserEntry},
        RequestSender,
    },
};

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
    c.is_ascii_alphabetic()
}

/// Validates character for id.
fn valid_id(c: char) -> bool {
    c.is_ascii_digit()
}

/// The flag worker flags and removes any grabbed post that matches with all of the tags in a `LineToken`.
/// The worker works with the supplied syntax and rules on e621's main site listed [here](https://e621.net/help/show/blacklist).
/// This ensures that the client-side blacklist works exactly the same as the server-side blacklist.
///
/// # Important
/// When doing any modifications to the worker, be sure to test the blacklist on client and server-side.
/// This will ensure that there aren't any unexpected behavior, or issues with the worker that weren't noticed.
/// A good thing to focus on is how many posts are blacklisted in total.
/// If the site says 236 posts are blacklisted, and the program is saying only 195 are blacklisted,
/// it's safe to assume there is a problem with how the worker is blacklisting posts.
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
                }
                TagType::Id(id) => {
                    if let Some(blacklisted_id) = id {
                        self.flag_id(*blacklisted_id, post.id, tag.negated);
                    }
                }
                TagType::User(_) => {
                    let user_id = tag
                        .name
                        .parse::<i64>()
                        .with_context(|e| {
                            error!("Failed to parse blacklisted user id: {}!", tag.name);
                            format!("{e}")
                        })
                        .unwrap();
                    self.flag_user(user_id, post.uploader_id, tag.negated);
                }
                TagType::None => {
                    if post_tags.iter().any(|e| e == tag.name.as_str()) {
                        self.raise_flag(tag.negated);
                    }
                }
            }
        }

        if self.is_negated_margin_met() {
            self.flagged = false;
        } else if self.is_margin_met() {
            self.flagged = true;
        }
    }

    fn is_negated_margin_met(&self) -> bool {
        self.negated_margin != 0 && self.negated_flags == self.negated_margin
    }

    fn is_margin_met(&self) -> bool {
        self.flags == self.margin
    }

    /// Raises either the `negated_flags` or `flags` by one depending on the value of `negated`.
    ///
    /// # Arguments
    ///
    /// * `negated`: The tag's negation.
    ///
    /// returns: ()
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
pub(crate) struct Blacklist {
    /// The blacklist parser which parses the blacklist and tokenizes it.
    blacklist_parser: BlacklistParser,
    /// All of the blacklist tokens after being parsed.
    blacklist_tokens: RootToken,
    /// Request sender used for getting user information.
    request_sender: RequestSender,
}

impl Blacklist {
    pub(crate) fn new(request_sender: RequestSender) -> Self {
        Blacklist {
            blacklist_parser: BlacklistParser::default(),
            blacklist_tokens: RootToken::default(),
            request_sender,
        }
    }

    pub(crate) fn parse_blacklist(&mut self, user_blacklist: String) -> &mut Blacklist {
        self.blacklist_parser = BlacklistParser::new(user_blacklist);
        self.blacklist_tokens = self.blacklist_parser.parse_blacklist();
        self
    }

    /// Goes through all of the blacklisted users and obtains there ID for the flagging system to cross examine with posts.
    pub(crate) fn cache_users(&mut self) {
        let tags: Vec<&mut TagToken> = self
            .blacklist_tokens
            .lines
            .iter_mut()
            .flat_map(|e| &mut e.tags)
            .collect();
        for tag in tags {
            if let TagType::User(Some(username)) = &tag.tag_type {
                let user: UserEntry = self
                    .request_sender
                    .get_entry_from_appended_id(username, "user");
                tag.name = format!("{}", user.id);
            }
        }
    }

    /// Checks of the blacklist is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.blacklist_tokens.lines.is_empty()
    }

    /// Filters through a set of posts, only retaining posts that aren't blacklisted.
    ///
    /// # Arguments
    ///
    /// * `posts`: Posts to filter through.
    ///
    /// returns: u16 of posts that were positively filtered
    pub(crate) fn filter_posts(&self, posts: &mut Vec<PostEntry>) -> u16 {
        let mut filtered: u16 = 0;
        for blacklist_line in &self.blacklist_tokens.lines {
            posts.retain(|e| {
                let mut flag_worker = FlagWorker::default();
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

        match filtered.cmp(&1) {
            Ordering::Less => trace!("No posts filtered..."),
            Ordering::Equal => trace!("Filtered {filtered} post with blacklist..."),
            Ordering::Greater => trace!("Filtered {filtered} posts with blacklist..."),
        }

        filtered
    }
}
