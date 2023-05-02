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


use std::cmp::Ordering;

use anyhow::Context;

use crate::e621::io::parser::BaseParser;
use crate::e621::sender::entries::{PostEntry, UserEntry};
use crate::e621::sender::RequestSender;

/// Root token which contains all the tokens of the blacklist.
#[derive(Default, Debug)]
struct RootToken {
    /// The total [LineToken]s from the root.
    lines: Vec<LineToken>,
}

/// A line token that contains all collected [`TagToken`]s from a parsed line.
#[derive(Debug, Default)]
struct LineToken {
    /// Total [TagToken] in the line.
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
    /// No rating.
    None,
    /// Safe rating.
    Safe,
    /// Questionable rating.
    Questionable,
    /// Explicit rating.
    Explicit,
}

/// A enum that contains what type the [TagToken] is.
///
/// The tag can be seen as four types: [Rating](TagType::Rating), [Id](TagType::Id), [User](TagType::User), and
/// [None](TagType::None).
#[derive(Debug)]
enum TagType {
    /// A post rating type.
    Rating(Rating),
    /// A post id type.
    Id(Option<i64>),
    /// A user type.
    User(Option<String>),
    /// The blacklisted score
    Score(Ordering, i32),
    /// No type.
    None,
}

/// Tag token that contains essential information about what is blacklisted.
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

    /// Parses each tag and collects them into a [`LineToken`].
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
    /// # Arguments
    ///
    /// * `token`: The special [TagToken] to parse.
    ///
    /// returns: ()
    ///
    /// # Errors
    ///
    /// An error can occur if 1) the `assert_eq` fails in its check or 2) if the [TagToken] name is not any of the matched
    /// values.
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
            "score" => {
                let ordering = self.get_ordering();
                let score = self.base_parser.consume_while(valid_score);
                token.tag_type = TagType::Score(
                    ordering,
                    score.parse::<i32>().unwrap(),
                );
            }
            _ => {
                self.base_parser.report_error(
                    format!("Unknown special tag identifier: {}", token.name).as_str(),
                );
            }
        };
    }

    /// Checks the value and create a new [Rating] from it.
    ///
    /// # Arguments
    ///
    /// * `value`: The value to check.
    ///
    /// returns: Rating
    fn get_rating(&self, value: &str) -> Rating {
        match value.to_lowercase().as_str() {
            "safe" | "s" => Rating::Safe,
            "questionable" | "q" => Rating::Questionable,
            "explicit" | "e" => Rating::Explicit,
            _ => Rating::None,
        }
    }

    /// Gets the ordering of the score.
    fn get_ordering(&mut self) -> Ordering {
        let order = self.base_parser.consume_while(valid_ordering);
        match order.as_str() {
            "<" => Ordering::Less,
            ">=" => Ordering::Greater, // This is greater than or equal, but ordering has no combination for that.
            _ => Ordering::Equal // Defaults to equal (e.g nothing happens).
        }
    }
}

/// Validates character for tag.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
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
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
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
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_rating(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// Validates character for ordering.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_ordering(c: char) -> bool {
    matches!(c, '<' | '>' | '=')
}

/// Validates character for score.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_score(c: char) -> bool {
    c.is_ascii_digit()
}

/// Validates character for id.
///
/// # Arguments
///
/// * `c`: The character to check.
///
/// returns: bool
fn valid_id(c: char) -> bool {
    c.is_ascii_digit()
}

/// A worker that checks and flags post based on a tag predicate, typically from the user's blacklist.
///
/// It works by comparing and removing any grabbed post that matches with all of the tags in a `LineToken`.
/// The worker works with the supplied syntax and rules on e621's main site listed [here](https://e621.net/help/show/blacklist).
/// This ensures that the client-side blacklist works exactly the same as the server-side blacklist.
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
    ///
    /// # Arguments
    ///
    /// * `tags`: The tags to calibrate flags for.
    fn set_flag_margin(&mut self, tags: &[TagToken]) {
        for tag in tags {
            if tag.negated {
                if let TagType::Score(_, _) = tag.tag_type {
                    // This is done because e621's blacklist itself doesn't handle scores that are negated, at
                    // least from my testing.
                    continue;
                }

                self.negated_margin += 1;
            } else {
                self.margin += 1;
            }
        }
    }

    /// Flags post based on blacklisted rating.
    ///
    /// # Arguments
    ///
    /// * `rating`: The blacklisted rating.
    /// * `post`: The post to check against.
    /// * `negated`: Whether the blacklisted rating is negated or not (this will determine if the rating whitelists the
    /// post or adds towards removing it from the download pool).
    fn flag_rating(&mut self, rating: &Rating, post: &PostEntry, negated: bool) {
        // A nice tuple hack to get around some massive nesting.
        match (rating, post.rating.as_str()) {
            (Rating::Safe, "s") | (Rating::Questionable, "q") | (Rating::Explicit, "e") => {
                self.raise_flag(negated);
            }
            (_, _) => {}
        }
    }

    /// Raises the flag and blacklists the post if its ID matches with the blacklisted ID.
    ///
    /// # Arguments
    ///
    /// * `id`: The blacklisted id to compare.
    /// * `post_id`: The post id to check against.
    /// * `negated`: Whether the blacklisted rating is negated or not (this will determine if the rating whitelists the
    /// post or adds towards removing it from the download pool).
    fn flag_id(&mut self, id: i64, post_id: i64, negated: bool) {
        if post_id == id {
            self.raise_flag(negated);
        }
    }

    /// Raises the flag and blacklists the post if the user who uploaded it is blacklisted.
    ///
    /// # Arguments
    ///
    /// * `user_id`: The blacklisted user id.
    /// * `uploader_id`: The user id to check against.
    /// * `negated`: Whether the blacklisted rating is negated or not (this will determine if the rating whitelists the
    /// post or adds towards removing it from the download pool).
    fn flag_user(&mut self, user_id: i64, uploader_id: i64, negated: bool) {
        if user_id == uploader_id {
            self.raise_flag(negated);
        }
    }

    /// Flags post based on it's score.
    ///
    /// # Arguments
    ///
    /// * `ordering`: The ordering of the score blacklisted (e.g <, >=).
    /// * `score`: The score to check and blacklist.
    /// * `post_score`: The post score to check against.
    fn flag_score(&mut self, ordering: &Ordering, score: &i32, post_score: i64, negated: bool) {
        match ordering {
            Ordering::Less => {
                if post_score < *score as i64 {
                    self.raise_flag(negated);
                }
            }
            Ordering::Greater => {
                if post_score >= *score as i64 {
                    self.raise_flag(negated);
                }
            }
            _ => {}
        }
    }

    /// Checks if a single post is blacklisted.
    ///
    /// # Arguments
    ///
    /// * `post`: The post to check.
    /// * `blacklist_line`: The blacklist tags to check the post against.
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
                        .with_context(|| {
                            format!("Failed to parse blacklisted user id: {}!", tag.name)
                        })
                        .unwrap();
                    self.flag_user(user_id, post.uploader_id, tag.negated);
                }
                TagType::Score(ordering, score) => {
                    self.flag_score(ordering, score, post.score.total, tag.negated);
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

    /// Returns true if the negated flags equals the negated margin, false otherwise.
    fn is_negated_margin_met(&self) -> bool {
        self.negated_margin != 0 && self.negated_flags == self.negated_margin
    }

    /// Returns true if the total flags equals the margin, false otherwise.
    fn is_margin_met(&self) -> bool {
        self.flags == self.margin
    }

    /// Raises either the `negated_flags` or `flags` by one depending on the value of `negated`.
    ///
    /// # Arguments
    ///
    /// * `negated`: The tag's negation.
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

    /// Parses the user blacklist.
    ///
    /// # Arguments
    ///
    /// * `user_blacklist`: The user blacklist to parse
    ///
    /// returns: &mut Blacklist
    pub(crate) fn parse_blacklist(&mut self, user_blacklist: String) -> &mut Blacklist {
        self.blacklist_parser = BlacklistParser::new(user_blacklist);
        self.blacklist_tokens = self.blacklist_parser.parse_blacklist();
        self
    }

    /// Caches user id into the tag name for quicker access during the blacklist checks.
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

    /// Checks if the blacklist is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.blacklist_tokens.lines.is_empty()
    }

    /// Filters through a set of posts, only retaining posts that aren't blacklisted.
    ///
    /// # Arguments
    ///
    /// * `posts`: Posts to filter through.
    ///
    /// returns: u16
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
