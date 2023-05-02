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

use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::tag::{Group, Tag, TagSearchType, TagType};
use crate::e621::io::{emergency_exit, Config, Login};
use crate::e621::sender::entries::{PoolEntry, PostEntry, SetEntry};
use crate::e621::sender::RequestSender;

/// A trait for implementing a conversion function for turning a type into a [Vec] of the same type
///
/// This can be used as a simple wrapping function for flattening an internal array of the type.
pub(crate) trait NewVec<T> {
    fn new_vec(value: T) -> Vec<Self>
    where
        Self: Sized;
}

/// A collection of values taken from a [PostEntry].
pub(crate) struct GrabbedPost {
    /// The url that leads to the file to download.
    url: String,
    /// The name of the file to download.
    name: String,
    /// The size of the file to download.
    file_size: i64,
}

impl GrabbedPost {
    /// The url that leads to the file to download.
    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    /// The name of the file to download.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The size of the file to download.
    pub(crate) fn file_size(&self) -> i64 {
        self.file_size
    }
}

impl NewVec<Vec<PostEntry>> for GrabbedPost {
    /// Creates a new [Vec] of type [GrabbedPost] from Vec of type [PostEntry]
    ///
    /// # Arguments
    ///
    /// * `vec`: The vector to be consumed and converted.
    ///
    /// returns: Vec<GrabbedPost, Global>
    fn new_vec(vec: Vec<PostEntry>) -> Vec<Self> {
        vec.into_iter()
            .map(|e| GrabbedPost::from((e, Config::get().naming_convention())))
            .collect()
    }
}

impl NewVec<(Vec<PostEntry>, &str)> for GrabbedPost {
    /// Creates a new [Vec] of type [GrabbedPost] from tuple contains types ([PostEntry], &str)
    ///
    /// Compared to the other overload, this version sets the name of the [GrabbedPost] and numbers them.
    ///
    /// # Arguments
    ///
    /// * `(vec, pool_name)`: A tuple containing the posts and the name of the pool associated with them.
    ///
    /// returns: Vec<GrabbedPost, Global>
    fn new_vec((vec, pool_name): (Vec<PostEntry>, &str)) -> Vec<Self> {
        vec.iter()
            .enumerate()
            .map(|(i, e)| GrabbedPost::from((e, pool_name, (i + 1) as u16)))
            .collect()
    }
}

impl From<(&PostEntry, &str, u16)> for GrabbedPost {
    /// Creates [GrabbedPost] from tuple of types (&[PostEntry], &str, u16)
    ///
    /// # Arguments
    ///
    /// * `(post, name, current_page)`: A tuple containing the post, name, and current page number of post.
    ///
    /// returns: GrabbedPost
    fn from((post, name, current_page): (&PostEntry, &str, u16)) -> Self {
        GrabbedPost {
            url: post.file.url.clone().unwrap(),
            name: format!("{} Page_{:05}.{}", name, current_page, post.file.ext),
            file_size: post.file.size,
        }
    }
}

impl From<(PostEntry, &str)> for GrabbedPost {
    /// Creates [GrabbedPost] from tuple of types ([PostEntry], &str)
    ///
    /// # Arguments
    ///
    /// * `(post, name_convention)`: A tuple containing the post, and naming convention of post.
    ///
    /// returns: GrabbedPost
    fn from((post, name_convention): (PostEntry, &str)) -> Self {
        match name_convention {
            "md5" => GrabbedPost {
                url: post.file.url.clone().unwrap(),
                name: format!("{}.{}", post.file.md5, post.file.ext),
                file_size: post.file.size,
            },
            "id" => GrabbedPost {
                url: post.file.url.clone().unwrap(),
                name: format!("{}.{}", post.id, post.file.ext),
                file_size: post.file.size,
            },
            _ => {
                emergency_exit("Incorrect naming convention!");
                GrabbedPost {
                    url: String::new(),
                    name: String::new(),
                    file_size: 0,
                }
            }
        }
    }
}

/// A trait for the shorten function, it allows for generic types to be the parameter.
pub(crate) trait Shorten<T> {
    /// Shortens a string by replacing a portion of it with a delimiter of type `T` and then returning the new string.
    fn shorten(&self, delimiter: T) -> String;
}

/// A set of posts with category and name.
pub(crate) struct PostCollection {
    /// The name of the set.
    name: String,
    /// The category of the set.
    category: String,
    /// The posts in the set.
    posts: Vec<GrabbedPost>,
}

impl PostCollection {
    /// Creates a new post collection.
    ///
    /// # Arguments
    ///
    /// * `name`: Name of collection.
    /// * `category`: Category of collection.
    /// * `posts`: Posts for collection.
    ///
    /// returns: PostCollection
    pub(crate) fn new(name: &str, category: &str, posts: Vec<GrabbedPost>) -> Self {
        PostCollection {
            name: name.to_string(),
            category: category.to_string(),
            posts,
        }
    }

    /// The name of the set.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// The category of the set.
    pub(crate) fn category(&self) -> &str {
        &self.category
    }

    /// The posts in the set.
    pub(crate) fn posts(&self) -> &Vec<GrabbedPost> {
        &self.posts
    }
}

impl Shorten<&str> for PostCollection {
    /// Shortens [PostCollection] name if it's greater than 25 characters and attaches the delimiter at the end.
    ///
    /// # Arguments
    ///
    /// * `delimiter`: What to replace the excess characters with.
    ///
    /// returns: String
    fn shorten(&self, delimiter: &str) -> String {
        if self.name.len() >= 25 {
            let mut short_name = self.name[0..25].to_string();
            short_name.push_str(delimiter);
            short_name
        } else {
            self.name.to_string()
        }
    }
}

impl Shorten<char> for PostCollection {
    /// Shortens [PostCollection] name if it's greater than 25 characters and attaches the delimiter at the end.
    ///
    /// # Arguments
    ///
    /// * `delimiter`: What to replace the excess characters with.
    ///
    /// returns: String
    fn shorten(&self, delimiter: char) -> String {
        if self.name.len() >= 25 {
            let mut short_name = self.name[0..25].to_string();
            short_name.push(delimiter);
            short_name
        } else {
            self.name.to_string()
        }
    }
}

impl From<(&SetEntry, Vec<GrabbedPost>)> for PostCollection {
    /// Creates [PostCollection] from tuple of types (&[SetEntry], [Vec]<[GrabbedPost]>).
    ///
    /// # Arguments
    ///
    /// * `(set, posts)`: The set and posts to make [PostCollection] from.
    ///
    /// returns: PostCollection
    fn from((set, posts): (&SetEntry, Vec<GrabbedPost>)) -> Self {
        PostCollection::new(&set.name, "Sets", posts)
    }
}

/// The total amount of pages the general search can search for.
const POST_SEARCH_LIMIT: u8 = 5;

/// Is a collector that grabs posts, categorizes them, and prepares them for the downloader to use in downloading.
pub(crate) struct Grabber {
    /// All grabbed posts.
    posts: Vec<PostCollection>,
    /// `RequestSender` for sending API calls.
    request_sender: RequestSender,
    /// Blacklist used to throwaway posts that contain tags the user may not want.
    blacklist: Option<Rc<RefCell<Blacklist>>>,
    /// Is grabber in safe mode or not
    safe_mode: bool,
}

impl Grabber {
    /// Creates a grabber for searching and grabbing posts.
    ///
    /// # Arguments
    ///
    /// * `request_sender`: The client to perform the searches.
    /// * `safe_mode`: Which mode the grabber will operate under.
    ///
    /// returns: Grabber
    pub(crate) fn new(request_sender: RequestSender, safe_mode: bool) -> Self {
        Grabber {
            posts: vec![PostCollection::new("Single Posts", "", Vec::new())],
            request_sender,
            blacklist: None,
            safe_mode,
        }
    }

    /// All grabbed posts.
    pub(crate) fn posts(&self) -> &Vec<PostCollection> {
        &self.posts
    }

    /// Sets the blacklist.
    ///
    /// # Arguments
    ///
    /// * `blacklist`: The new blacklist
    pub(crate) fn set_blacklist(&mut self, blacklist: Rc<RefCell<Blacklist>>) {
        if !blacklist.borrow_mut().is_empty() {
            self.blacklist = Some(blacklist);
        }
    }

    /// Sets safe mode.
    ///
    /// If set true, the grabber will go into safe mode and grab only safe posts,
    /// false will grab questionable and explicit posts.
    ///
    /// # Arguments
    ///
    /// * `mode`: Which mode to run in
    pub(crate) fn set_safe_mode(&mut self, mode: bool) {
        self.safe_mode = mode;
    }

    /// Grabs favorites from the user's favorites
    pub(crate) fn grab_favorites(&mut self) {
        let login = Login::get();
        if !login.username().is_empty() && login.download_favorites() {
            let tag = format!("fav:{}", login.username());
            let posts = self.search(&tag, &TagSearchType::Special);
            self.posts
                .push(PostCollection::new(&tag, "", GrabbedPost::new_vec(posts)));
            info!(
                "{} grabbed!",
                console::style(format!("\"{tag}\"")).color256(39).italic()
            );
        }
    }

    /// Grabs new posts by the given tag.
    ///
    /// # Arguments
    ///
    /// * `groups`: The group of tags to search for.
    pub(crate) fn grab_posts_by_tags(&mut self, groups: &[Group]) {
        let tags: Vec<&Tag> = groups.iter().flat_map(|e| e.tags()).collect();
        for tag in tags {
            self.grab_by_tag_type(tag);
        }
    }

    /// Returns the single post [PostCollection].
    fn single_post_collection(&mut self) -> &mut PostCollection {
        self.posts.first_mut().unwrap() // It is guaranteed that the first collection is the single post collection.
    }

    /// Adds a single post to the single post [PostCollection]
    ///
    /// # Arguments
    ///
    /// * `entry`: The entry to add to the collection.
    /// * `id`: The id that's used for debugging.
    fn add_single_post(&mut self, entry: PostEntry, id: i64) {
        match entry.file.url {
            None => warn!(
                "Post with ID {} has no URL!",
                console::style(format!("\"{id}\"")).color256(39).italic()
            ),
            Some(_) => {
                let grabbed_post = GrabbedPost::from((entry, Config::get().naming_convention()));
                self.single_post_collection().posts.push(grabbed_post);
                info!(
                    "Post with ID {} grabbed!",
                    console::style(format!("\"{id}\"")).color256(39).italic()
                );
            }
        }
    }

    /// Searches and grabs post based on the tag given.
    ///
    /// # Arguments
    ///
    /// * `tag`: The tag to search for.
    fn grab_by_tag_type(&mut self, tag: &Tag) {
        match tag.tag_type() {
            TagType::Pool => self.grab_pool(tag),
            TagType::Set => self.grab_set(tag),
            TagType::Post => self.grab_post(tag),
            TagType::General | TagType::Artist => self.grab_general(tag),
            TagType::Unknown => unreachable!(),
        };
    }

    /// Grabs general posts based on the given tag.
    ///
    /// # Arguments
    ///
    /// * `tag`: The tag to search for.
    fn grab_general(&mut self, tag: &Tag) {
        let posts = self.get_posts_from_tag(tag);
        self.posts.push(PostCollection::new(
            tag.name(),
            "General Searches",
            GrabbedPost::new_vec(posts),
        ));
        info!(
            "{} grabbed!",
            console::style(format!("\"{}\"", tag.name()))
                .color256(39)
                .italic()
        );
    }

    /// Grabs single post based on the given tag.
    ///
    /// # Arguments
    ///
    /// * `tag`: The tag to search for.
    fn grab_post(&mut self, tag: &Tag) {
        let entry: PostEntry = self
            .request_sender
            .get_entry_from_appended_id(tag.name(), "single");
        let id = entry.id;

        if self.safe_mode {
            match entry.rating.as_str() {
                "s" => {
                    self.add_single_post(entry, id);
                }
                _ => {
                    info!(
                        "Skipping Post: {} due to being explicit or questionable",
                        console::style(format!("\"{id}\"")).color256(39).italic()
                    );
                }
            }
        } else {
            self.add_single_post(entry, id);
        }
    }

    /// Grabs a set based on the given tag.
    ///
    /// # Arguments
    ///
    /// * `tag`: The tag to search for.
    fn grab_set(&mut self, tag: &Tag) {
        let entry: SetEntry = self
            .request_sender
            .get_entry_from_appended_id(tag.name(), "set");

        // Grabs posts from IDs in the set entry.
        let posts = self.search(&format!("set:{}", entry.shortname), &TagSearchType::Special);
        self.posts
            .push(PostCollection::from((&entry, GrabbedPost::new_vec(posts))));

        info!(
            "{} grabbed!",
            console::style(format!("\"{}\"", entry.name))
                .color256(39)
                .italic()
        );
    }

    /// Grabs pool based on the given tag.
    ///
    /// # Arguments
    ///
    /// * `tag`: The tag to search for.
    fn grab_pool(&mut self, tag: &Tag) {
        let mut entry: PoolEntry = self
            .request_sender
            .get_entry_from_appended_id(tag.name(), "pool");
        let name = &entry.name;
        let mut posts = self.search(&format!("pool:{}", entry.id), &TagSearchType::Special);

        // Updates entry post ids in case any posts were filtered in the search.
        entry
            .post_ids
            .retain(|id| posts.iter().any(|post| post.id == *id));

        // Sorts the pool to the original order given by entry.
        Self::sort_pool_by_id(&entry, &mut posts);

        self.posts.push(PostCollection::new(
            name,
            "Pools",
            GrabbedPost::new_vec((posts, name.as_ref())),
        ));

        info!(
            "{} grabbed!",
            console::style(format!("\"{name}\"")).color256(39).italic()
        );
    }

    /// Sorts a pool by id based on the supplied [PoolEntry].
    ///
    /// # Arguments
    ///
    /// * `entry`: The [PoolEntry] to check ids against
    /// * `posts`: The [PostEntry] array to sort
    fn sort_pool_by_id(entry: &PoolEntry, posts: &mut [PostEntry]) {
        for (i, id) in entry.post_ids.iter().enumerate() {
            if posts[i].id != *id {
                let correct_index = posts.iter().position(|e| e.id == *id).unwrap();
                posts.swap(i, correct_index);
            }
        }
    }

    /// Searches and grabs posts using the given tag.
    ///
    /// # Arguments
    ///
    /// * `tag`: The tag to use for the search.
    ///
    /// returns: Vec<PostEntry, Global>
    fn get_posts_from_tag(&self, tag: &Tag) -> Vec<PostEntry> {
        self.search(tag.name(), tag.search_type())
    }

    /// Performs a search where it grabs posts.
    ///
    /// Depending on the given [TagSearchType], the way posts are grabs will be different.
    /// - [General](TagSearchType::General) will search through pages only up to the [POST_SEARCH_LIMIT]
    /// - [Special](TagSearchType::Special) will search repeatedly until there are no pages left to grab.
    ///
    /// # Arguments
    ///
    /// * `searching_tag`: The tag used for the search.
    /// * `tag_search_type`: The type of search to happen.
    ///
    /// returns: Vec<PostEntry, Global>
    fn search(&self, searching_tag: &str, tag_search_type: &TagSearchType) -> Vec<PostEntry> {
        let mut posts: Vec<PostEntry> = Vec::new();
        let mut filtered = 0;
        let mut invalid_posts = 0;
        match tag_search_type {
            TagSearchType::General => {
                posts = Vec::with_capacity(320 * POST_SEARCH_LIMIT as usize);
                self.general_search(searching_tag, &mut posts, &mut filtered, &mut invalid_posts);
            }
            TagSearchType::Special => {
                self.special_search(searching_tag, &mut posts, &mut filtered, &mut invalid_posts);
            }
            TagSearchType::None => {}
        }

        if filtered > 0 {
            info!(
                "Filtered {} total blacklisted posts from search...",
                console::style(filtered).cyan().italic()
            );
        }

        if invalid_posts > 0 {
            info!(
                "Filtered {} total invalid posts from search...",
                console::style(invalid_posts).cyan().italic()
            );
        }

        posts
    }

    /// Performs a special search to grab posts.
    ///
    /// The difference between special/general searches are this.
    /// - Special searches aim to keep grabbing posts until there are not posts left to grab.
    /// - General searches aim to grab only a few pages of posts (commonly 320 posts per page). You can refer to the
    /// [POST_SEARCH_LIMIT] for the current search limit of the general search.
    ///
    /// # Arguments
    ///
    /// * `searching_tag`: The tag to search for.
    /// * `posts`:  The posts [Vec] to add searched posts into.
    /// * `filtered`: The total amount of posts filtered.
    /// * `invalid_posts`: The total amount of posts invalid by the [Blacklist].
    fn special_search(
        &self,
        searching_tag: &str,
        posts: &mut Vec<PostEntry>,
        filtered: &mut u16,
        invalid_posts: &mut u16,
    ) {
        let mut page = 1;

        loop {
            let mut searched_posts = self.request_sender.bulk_search(searching_tag, page).posts;
            if searched_posts.is_empty() {
                break;
            }

            *filtered += self.filter_posts_with_blacklist(&mut searched_posts);
            *invalid_posts += Self::remove_invalid_posts(&mut searched_posts);

            searched_posts.reverse();
            posts.append(&mut searched_posts);
            page += 1;
        }
    }

    /// Performs a general search to grab posts.
    ///
    /// The difference between special/general searches are this.
    /// - Special searches aim to keep grabbing posts until there are not posts left to grab.
    /// - General searches aim to grab only a few pages of posts (commonly 320 posts per page). You can refer to the
    /// [POST_SEARCH_LIMIT] for the current search limit of the general search.
    ///
    /// # Arguments
    ///
    /// * `searching_tag`: The tag to search for.
    /// * `posts`:  The posts [Vec] to add searched posts into.
    /// * `filtered`: The total amount of posts filtered.
    /// * `invalid_posts`: The total amount of posts invalid by the [Blacklist].
    fn general_search(
        &self,
        searching_tag: &str,
        posts: &mut Vec<PostEntry>,
        filtered: &mut u16,
        invalid_posts: &mut u16,
    ) {
        for page in 1..POST_SEARCH_LIMIT {
            let mut searched_posts: Vec<PostEntry> = self
                .request_sender
                .bulk_search(searching_tag, page as u16)
                .posts;
            if searched_posts.is_empty() {
                break;
            }

            *filtered += self.filter_posts_with_blacklist(&mut searched_posts);
            *invalid_posts += Self::remove_invalid_posts(&mut searched_posts);

            searched_posts.reverse();
            posts.append(&mut searched_posts);
        }
    }

    /// Checks through posts and removes any that violets the blacklist.
    ///
    /// # Arguments
    ///
    /// * `posts`: The posts to check
    ///
    /// returns: u16
    fn filter_posts_with_blacklist(&self, posts: &mut Vec<PostEntry>) -> u16 {
        if self.request_sender.is_authenticated() {
            if let Some(ref blacklist) = self.blacklist {
                return blacklist.borrow_mut().filter_posts(posts);
            }
        }

        0
    }

    /// Removes invalid posts (e.g posts with no urls, or invalid properties).
    ///
    /// Sometimes, even if a post is available, the url for it isn't; To handle this, the [Vec]<[PostEntry]> will retain only
    /// the posts that has an available url. So far, the only check needed is the url check, since if the url is [None],
    /// the entire post is [None].
    ///
    /// # Arguments
    ///
    /// * `posts`: Posts to check.
    ///
    /// returns: u16
    fn remove_invalid_posts(posts: &mut Vec<PostEntry>) -> u16 {
        let mut invalid_posts = 0;
        posts.retain(|e| {
            if !e.flags.deleted && e.file.url.is_some() {
                true
            } else {
                invalid_posts += 1;
                false
            }
        });

        Self::log_invalid_posts(&invalid_posts);

        invalid_posts
    }

    /// Traces invalid posts to the log file.
    ///
    /// # Arguments
    ///
    /// * `invalid_posts`: The total count of invalid posts.
    fn log_invalid_posts(invalid_posts: &u16) {
        match invalid_posts.cmp(&1) {
            Ordering::Less => {}
            Ordering::Equal => {
                trace!(
                    "A post was filtered for being invalid (due to the user not being logged in)"
                );
                trace!("A post was filtered by e621...");
            }
            Ordering::Greater => {
                trace!("{} posts were filtered for being invalid (due to the user not being logged in)", invalid_posts);
                trace!("{} posts had to be filtered by e621/e926...", invalid_posts,);
            }
        }
    }
}
