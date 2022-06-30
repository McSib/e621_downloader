use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use super::io::tag::{UserTags, StringTag};
use crate::e621::blacklist::Blacklist;
use crate::e621::io::{
    emergency_exit,
    Config,
    Login,
};
use crate::e621::sender::entries::{
    PoolEntry,
    PostEntry,
    SetEntry,
};
use crate::e621::sender::RequestSender;

/// `PostEntry` that was grabbed and converted into `GrabbedPost`, it contains only the necessary information for downloading the post.
pub struct GrabbedPost {
    /// The url that leads to the file to download.
    url: String,
    /// The name of the file to download.
    name: String,
    /// The size of the file to download.
    file_size: i64,
}

impl GrabbedPost {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn file_size(&self) -> i64 {
        self.file_size
    }

    /// Takes an array of `PostEntry`s and converts it into an array of `GrabbedPost`s.
    pub fn entry_to_vec(vec: Vec<PostEntry>) -> Vec<GrabbedPost> {
        let config = Config::get_config().unwrap();
        vec.into_iter()
            .map(|e| GrabbedPost::from(e, config.naming_convention()))
            .collect()
    }

    /// Takes an array of `PostEntry`s and converts it into an array of `GrabbedPost`s for pools.
    pub fn entry_to_pool_vec(vec: Vec<PostEntry>, pool_name: &str) -> Vec<GrabbedPost> {
        vec.iter()
            .enumerate()
            .map(|(i, e)| GrabbedPost::from_entry_to_pool(e, pool_name, (i + 1) as u16))
            .collect()
    }

    /// Converts `PostEntry` to `Self`.
    pub fn from_entry_to_pool(post: &PostEntry, name: &str, current_page: u16) -> Self {
        GrabbedPost {
            url: post.file.url.clone().unwrap(),
            name: format!("{} Page_{:05}.{}", name, current_page, post.file.ext),
            file_size: post.file.size,
        }
    }

    /// Converts `PostEntry` to `Self`.
    fn from(post: PostEntry, name_convention: &str) -> Self {
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

/// A set of posts with category and name.
pub struct PostCollection {
    /// The name of the set.
    name: String,
    /// The category of the set.
    category: String,
    /// The posts in the set.
    posts: Vec<GrabbedPost>,
}

impl PostCollection {
    pub fn new(name: &str, category: &str, posts: Vec<GrabbedPost>) -> Self {
        PostCollection {
            name: name.to_string(),
            category: category.to_string(),
            posts,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    pub fn posts(&self) -> &Vec<GrabbedPost> {
        &self.posts
    }

    /// Converts `SetEntry` to `Self`.
    pub fn from_set(set: &SetEntry, posts: Vec<GrabbedPost>) -> Self {
        PostCollection::new(&set.name, "Sets", posts)
    }
}

/// Grabs all posts under a set of searching tags.
pub struct Grabber {
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
    /// Creates new instance of `Self`.
    pub fn new(request_sender: RequestSender, safe_mode: bool) -> Self {
        Grabber {
            posts: vec![PostCollection::new("Single Posts", "", Vec::new())],
            request_sender,
            blacklist: None,
            safe_mode,
        }
    }

    pub fn posts(&self) -> &Vec<PostCollection> {
        &self.posts
    }

    /// Sets the blacklist.
    pub fn set_blacklist(&mut self, blacklist: Rc<RefCell<Blacklist>>) {
        if !blacklist.borrow_mut().is_empty() {
            self.blacklist = Some(blacklist);
        }
    }

    pub fn set_safe_mode(&mut self, mode: bool) {
        self.safe_mode = mode;
    }

    /// If the user supplies login information, this will grabbed the favorites from there account.
    pub fn grab_favorites(&mut self) {
        let login = Login::load().unwrap_or_else(|e| {
			error!("Unable to load `login.json`. Error: {}", e);
			warn!("The program will use default values, but it is highly recommended to check your login.json file to \
			       ensure that everything is correct.");
			Login::default()
		});
        if !login.username().is_empty() && login.download_favorites() {
            let tag_str = format!("fav:{}", login.username());
            let posts = self.special_search(tag_str.as_str());
            self.posts.push(PostCollection::new(
                &tag_str,
                "",
                GrabbedPost::entry_to_vec(posts),
            ));
            info!(
                "{} grabbed!",
                console::style(format!("\"{}\"", tag_str))
                    .color256(39)
                    .italic()
            );
        }
    }

    /// Iterates through tags and perform searches for each, grabbing them and storing them for later download.
    pub fn grab_posts_by_tags(&mut self, user_tags: &UserTags) {
        let config = Config::get_config().unwrap();
        for general_tag in user_tags.general() {
            let posts = self.get_posts_from_tag(general_tag, true);
            self.posts.push(PostCollection::new(
                general_tag.name(),
                "General Searches",
                GrabbedPost::entry_to_vec(posts),
            ));
            info!(
                "{} grabbed!",
                console::style(format!("\"{}\"", general_tag.name()))
                    .color256(39)
                    .italic()
            );
        }

        for artist_tag in user_tags.artists() {
            let posts = self.get_posts_from_tag(artist_tag, false);
            self.posts.push(PostCollection::new(
                artist_tag.name(),
                "General Searches",
                GrabbedPost::entry_to_vec(posts),
            ));
            info!(
                "{} grabbed!",
                console::style(format!("\"{}\"", artist_tag.name()))
                    .color256(39)
                    .italic()
            );
        }

        for post_tag in user_tags.single_posts() {
            let mut add_post = |entry: PostEntry, id: i64| {
                self.posts
                    .first_mut()
                    .unwrap()
                    .posts
                    .push(GrabbedPost::from(entry, config.naming_convention()));

                info!(
                    "Post with ID {} grabbed!",
                    console::style(format!("\"{}\"", id)).color256(39).italic()
                );
            };

            let entry: PostEntry = self
                .request_sender
                .get_entry_from_appended_id(&format!("{}", post_tag.id()), "single");
            let id = entry.id;

            if self.safe_mode {
                match entry.rating.as_str() {
                    "s" => {
                        add_post(entry, id);
                    }
                    _ => {
                        info!(
                            "Skipping Post: {}",
                            console::style(format!("\"{}\"", id)).color256(39).italic()
                        );
                        info!("Post was found to be explicit or questionable...")
                    }
                }
            } else {
                add_post(entry, id);
            }
        }

        for pool_tag in user_tags.pools() {
            let mut entry: PoolEntry = self
                .request_sender
                .get_entry_from_appended_id(&format!("{}", pool_tag.id()), "pool");
            let name = &entry.name;
            let mut posts = self.special_search(&format!("pool:{}", entry.id));

            // Updates entry post ids in case any posts were filtered in the search.
            entry
                .post_ids
                .retain(|id| posts.iter().any(|post| post.id == *id));

            // Sorts the pool to the original order given by entry.
            for (i, id) in entry.post_ids.iter().enumerate() {
                if posts[i].id != *id {
                    let correct_index = posts.iter().position(|e| e.id == *id).unwrap();
                    posts.swap(i, correct_index);
                }
            }

            self.posts.push(PostCollection::new(
                name,
                "Pools",
                GrabbedPost::entry_to_pool_vec(posts, name),
            ));

            info!(
                "{} grabbed!",
                console::style(format!("\"{}\"", name))
                    .color256(39)
                    .italic()
            );
        }

        for set_tag in user_tags.sets() {
            let entry: SetEntry = self
                .request_sender
                .get_entry_from_appended_id(&format!("{}", set_tag.id()), "set");

            // Grabs posts from IDs in the set entry.
            let posts = self.special_search(&format!("set:{}", entry.shortname));
            self.posts.push(PostCollection::from_set(
                &entry,
                GrabbedPost::entry_to_vec(posts),
            ));

            info!(
                "{} grabbed!",
                console::style(format!("\"{}\"", entry.name))
                    .color256(39)
                    .italic()
            );
        }
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&self, tag: &StringTag, is_general: bool) -> Vec<PostEntry> {
        if is_general {
            self.general_search(tag.name())
        } else {
            self.special_search(tag.name())
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(&self, searching_tag: &str) -> Vec<PostEntry> {
        let limit: u16 = 5;
        let mut posts: Vec<PostEntry> = Vec::with_capacity(320 * limit as usize);
        let mut filtered = 0;
        let mut invalid_posts = 0;
        for page in 1..limit {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page).posts;
            if searched_posts.is_empty() {
                break;
            }

            filtered += self.filter_posts_with_blacklist(&mut searched_posts);
            invalid_posts += self.remove_invalid_posts(&mut searched_posts);

            searched_posts.reverse();
            posts.append(&mut searched_posts);
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

    /// Performs a special search that grabs all posts tied to the searching tag.
    fn special_search(&self, searching_tag: &str) -> Vec<PostEntry> {
        let mut page: u16 = 1;
        let mut posts: Vec<PostEntry> = vec![];
        let mut filtered = 0;
        let mut invalid_posts = 0;
        loop {
            let mut searched_posts = self.request_sender.bulk_search(searching_tag, page).posts;
            if searched_posts.is_empty() {
                break;
            }

            filtered += self.filter_posts_with_blacklist(&mut searched_posts);
            invalid_posts += self.remove_invalid_posts(&mut searched_posts);

            searched_posts.reverse();
            posts.append(&mut searched_posts);
            page += 1;
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

    /// Scans through array of posts and removes any that violets the blacklist.
    fn filter_posts_with_blacklist(&self, posts: &mut Vec<PostEntry>) -> u16 {
        if self.request_sender.is_authenticated() {
            if let Some(ref blacklist) = self.blacklist {
                return blacklist.borrow_mut().filter_posts(posts);
            }
        }

        0
    }

    /// Removes invalid posts, this is dependant on if the file url is null or if the post was deleted.
    fn remove_invalid_posts(&self, posts: &mut Vec<PostEntry>) -> u16 {
        // Sometimes, even if a post is available, the url for it isn't;
        // To handle this, the vector will retain only the posts that has an available url.
        let mut invalid_posts = 0;
        posts.retain(|e| {
            if !e.flags.deleted && e.file.url != None {
                true
            } else {
                invalid_posts += 1;
                false
            }
        });

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

        invalid_posts
    }
}
