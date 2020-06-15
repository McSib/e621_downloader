extern crate reqwest;
extern crate serde_json;

use serde_json::Value;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::tag::{Group, Tag, TagCategory, TagType};
use crate::e621::io::Login;
use crate::e621::sender::{
    BulkPostEntry, PoolEntry, PostEntry, RequestSender, SetEntry, UserEntry,
};

/// `PostEntry` that was grabbed and converted into `GrabbedPost`, it contains only the necessary information for downloading the post.
pub struct GrabbedPost {
    /// The url that leads to the file to download.
    pub file_url: String,
    /// The name of the file to download.
    pub file_name: String,
    /// The size of the file to download.
    pub file_size: i64,
}

impl GrabbedPost {
    /// Takes an array of `PostEntry`s and converts it into an array of `GrabbedPost`s.
    pub fn entry_to_vec(vec: Vec<PostEntry>) -> Vec<GrabbedPost> {
        let mut temp_vec = Vec::with_capacity(vec.len());
        for post in vec {
            temp_vec.push(GrabbedPost::from(post));
        }
        temp_vec
    }

    /// Takes an array of `PostEntry`s and converts it into an array of `GrabbedPost`s for pools.
    pub fn entry_to_pool_vec(vec: Vec<PostEntry>, pool_name: &str) -> Vec<GrabbedPost> {
        let mut temp_vec = Vec::with_capacity(vec.len());
        for (i, post) in vec.iter().enumerate() {
            temp_vec.push(GrabbedPost::from_entry_to_pool(
                post,
                pool_name,
                (i + 1) as u16,
            ));
        }
        temp_vec
    }

    /// Converts `PostEntry` to `Self`.
    pub fn from_entry_to_pool(post: &PostEntry, name: &str, current_page: u16) -> Self {
        GrabbedPost {
            file_url: post.file.url.clone().unwrap(),
            file_name: format!("{}{:04}.{}", name, current_page, post.file.ext),
            file_size: post.file.size,
        }
    }
}

impl From<PostEntry> for GrabbedPost {
    /// Converts `PostEntry` to `Self`.
    fn from(post: PostEntry) -> Self {
        GrabbedPost {
            file_url: post.file.url.clone().unwrap(),
            file_name: format!("{}.{}", post.file.md5, post.file.ext),
            file_size: post.file.size,
        }
    }
}

/// A set of posts with category and name.
pub struct PostSet {
    /// The name of the set.
    pub set_name: String,
    /// The category of the set.
    pub category: String,
    /// The posts in the set.
    pub posts: Vec<GrabbedPost>,
}

impl PostSet {
    pub fn new(set_name: &str, category: &str, posts: Vec<GrabbedPost>) -> Self {
        PostSet {
            set_name: set_name.to_string(),
            category: category.to_string(),
            posts,
        }
    }

    /// Converts `SetEntry` to `Self`.
    pub fn from_set(set: &SetEntry, posts: Vec<GrabbedPost>) -> Self {
        PostSet::new(&set.name, "Sets", posts)
    }
}

/// Grabs all posts under a set of searching tags.
pub struct Grabber {
    /// All grabbed posts.
    pub grabbed_posts: Vec<PostSet>,
    /// All grabbed single posts.
    pub grabbed_single_posts: PostSet,
    /// `RequestSender` for sending API calls.
    request_sender: RequestSender,
    /// Blacklist used to throwaway posts that contain tags the user may not want.
    blacklist: Option<Blacklist>,
}

impl Grabber {
    /// Creates new instance of `Self`.
    pub fn new(request_sender: RequestSender) -> Self {
        Grabber {
            grabbed_posts: Vec::new(),
            grabbed_single_posts: PostSet::new("Single Posts", "", Vec::new()),
            request_sender,
            blacklist: None,
        }
    }

    /// Gets posts on creation using `groups` and searching with `request_sender`.
    pub fn from_tags(groups: &[Group], request_sender: RequestSender) -> Grabber {
        let mut grabber = Grabber::new(request_sender);
        grabber.grab_blacklist();
        grabber.grab_favorites();
        grabber.grab_tags(groups);
        grabber
    }

    /// If login information is supplied, the connector will log into the supplied account and obtain it's blacklist.
    /// This should be the only time the connector ever logs in.
    pub fn grab_blacklist(&mut self) {
        let login = Login::load().unwrap_or_else(|e| {
            println!("Unable to load `login.json`. Error: {}", e);
            println!("The program will use default values, but it is highly recommended to check your login.json file to ensure that everything is correct.");
            Login::default()
        });
        if !login.is_empty() {
            let user: UserEntry = self.request_sender.get_blacklist(&login);
            let blacklist_string = user
                .blacklisted_tags
                .unwrap()
                .trim_matches('\"')
                .to_string();
            println!("{}", blacklist_string);
            let blacklist_entries: Vec<String> =
                blacklist_string.lines().map(|e| e.to_string()).collect();
            self.blacklist = if !blacklist_entries.is_empty() {
                Some(Blacklist::new(blacklist_entries))
            } else {
                None
            };
        }
    }

    /// If the user supplies login information, this will grabbed the favorites from there account.
    pub fn grab_favorites(&mut self) {
        let login = Login::load().unwrap_or_else(|e| {
            println!("Unable to load `login.json`. Error: {}", e);
            println!("The program will use default values, but it is highly recommended to check your login.json file to ensure that everything is correct.");
            Login::default()
        });
        if !login.username.is_empty() && login.download_favorites {
            let tag_str = format!("fav:{}", login.username);
            let posts = self.special_search(tag_str.as_str());
            self.grabbed_posts
                .push(PostSet::new(&tag_str, "", GrabbedPost::entry_to_vec(posts)));
            println!("\"{}\" grabbed!", tag_str);
        }
    }

    /// Iterates through tags and perform searches for each, grabbing them and storing them for later download.
    pub fn grab_tags(&mut self, groups: &[Group]) {
        for group in groups {
            for tag in &group.tags {
                match tag.tag_type {
                    TagType::Pool => {
                        let entry: PoolEntry = self
                            .request_sender
                            .get_entry_from_appended_id(&tag.raw, "pool");
                        let name = &entry.name;
                        let posts = self.special_search(&format!("pool:{}", entry.id));
                        self.grabbed_posts.push(PostSet::new(
                            name,
                            "Pools",
                            GrabbedPost::entry_to_pool_vec(posts, name),
                        ));

                        println!("\"{}\" grabbed!", name);
                    }
                    TagType::Set => {
                        let entry: SetEntry = self
                            .request_sender
                            .get_entry_from_appended_id(&tag.raw, "set");
                        // Grabs posts from IDs in the set entry.
                        let posts = self.special_search(&format!("set:{}", entry.shortname));
                        self.grabbed_posts
                            .push(PostSet::from_set(&entry, GrabbedPost::entry_to_vec(posts)));

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    TagType::Post => {
                        let entry: PostEntry = self
                            .request_sender
                            .get_entry_from_appended_id(&tag.raw, "single");
                        let id = entry.id;
                        self.grabbed_single_posts
                            .posts
                            .push(GrabbedPost::from(entry));

                        println!("Post with ID \"{}\" grabbed!", id);
                    }
                    TagType::General | TagType::Artist => {
                        let posts = self.get_posts_from_tag(tag);
                        self.grabbed_posts.push(PostSet::new(
                            &tag.raw,
                            "General Searches",
                            GrabbedPost::entry_to_vec(posts),
                        ));
                        println!("\"{}\" grabbed!", tag.raw);
                    }
                    TagType::None => unreachable!(),
                };
            }
        }
    }

    #[deprecated(
        since = "1.5.6",
        note = "Avoid this function as it uses the old API system."
    )]
    /// Grabs all posts from pool.
    pub fn get_posts_from_pool(&self, id: &str) -> (String, Vec<PostEntry>) {
        let mut page: u16 = 1;
        let mut name = String::new();
        let mut posts: Vec<PostEntry> = vec![];
        // TODO: Since all posts are now grabbed when the pool entry is grabbed, looping is no longer needed for pages
        loop {
            let mut searched_pool: PoolEntry = self.request_sender.get_pool_entry(id, page);
            if searched_pool.post_ids.is_empty() {
                break;
            }

            if name.is_empty() {
                name = searched_pool.name.clone();
            }

            // Sets the capacity to the total amount of posts in the pool
            // so the next pages to add will be done quicker.
            if posts.capacity() == 0 {
                posts = Vec::with_capacity(searched_pool.post_count as usize);
            }

            // posts.append(&mut searched_pool.posts);
            page += 1;
        }

        (name, posts)
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&mut self, tag: &Tag) -> Vec<PostEntry> {
        match tag.search_type {
            TagCategory::General => self.general_search(&tag.raw),
            TagCategory::Special => self.special_search(&tag.raw),
            TagCategory::None => unreachable!(),
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(&mut self, searching_tag: &str) -> Vec<PostEntry> {
        let limit: u16 = 5;
        let mut posts: Vec<PostEntry> = Vec::with_capacity(320 * limit as usize);
        for page in 1..limit {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page).posts;
            if searched_posts.is_empty() {
                break;
            }

            self.filter_posts_with_blacklist(&mut searched_posts);
            self.remove_invalid_posts(&mut searched_posts);

            searched_posts.reverse();
            posts.append(&mut searched_posts);
        }

        posts
    }

    /// Performs a special search that grabs all posts tied to the searching tag.
    fn special_search(&self, searching_tag: &str) -> Vec<PostEntry> {
        let mut page: u16 = 1;
        let mut posts: Vec<PostEntry> = vec![];
        loop {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page).posts;
            if searched_posts.is_empty() {
                break;
            }

            self.filter_posts_with_blacklist(&mut searched_posts);
            self.remove_invalid_posts(&mut searched_posts);

            searched_posts.reverse();
            posts.append(&mut searched_posts);
            page += 1;
        }

        posts
    }

    fn filter_posts_with_blacklist(&self, posts: &mut Vec<PostEntry>) {
        if self.request_sender.is_authenticated() {
            if let Some(ref e) = self.blacklist {
                e.filter_posts(posts, &self.request_sender);
            }
        }
    }

    fn remove_invalid_posts(&self, posts: &mut Vec<PostEntry>) {
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

        if invalid_posts > 0 {
            println!(
                "Over {} posts had to be filtered because the file url wasn't available.",
                invalid_posts
            )
        }
    }
}
