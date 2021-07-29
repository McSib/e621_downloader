use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::tag::{Group, Tag, TagCategory, TagType};
use crate::e621::io::Login;
use crate::e621::sender::entries::{PoolEntry, PostEntry, SetEntry};
use crate::e621::sender::RequestSender;

/// `PostEntry` that was grabbed and converted into `GrabbedPost`, it contains only the necessary information for downloading the post.
pub struct GrabbedPost {
    /// The url that leads to the file to download.
    pub url: String,
    /// The name of the file to download.
    pub name: String,
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
            url: post.file.url.clone().unwrap(),
            name: format!("{} Page_{:04}.{}", name, current_page, post.file.ext),
            file_size: post.file.size,
        }
    }
}

impl From<PostEntry> for GrabbedPost {
    /// Converts `PostEntry` to `Self`.
    fn from(post: PostEntry) -> Self {
        GrabbedPost {
            url: post.file.url.clone().unwrap(),
            name: format!("{}.{}", post.file.md5, post.file.ext),
            file_size: post.file.size,
        }
    }
}

/// A set of posts with category and name.
pub struct PostCollection {
    /// The name of the set.
    pub name: String,
    /// The category of the set.
    pub category: String,
    /// The posts in the set.
    pub posts: Vec<GrabbedPost>,
}

impl PostCollection {
    pub fn new(name: &str, category: &str, posts: Vec<GrabbedPost>) -> Self {
        PostCollection {
            name: name.to_string(),
            category: category.to_string(),
            posts,
        }
    }

    /// Converts `SetEntry` to `Self`.
    pub fn from_set(set: &SetEntry, posts: Vec<GrabbedPost>) -> Self {
        PostCollection::new(&set.name, "Sets", posts)
    }
}

/// Grabs all posts under a set of searching tags.
pub struct Grabber {
    /// All grabbed posts.
    pub posts: Vec<PostCollection>,
    /// `RequestSender` for sending API calls.
    request_sender: RequestSender,
    /// Blacklist used to throwaway posts that contain tags the user may not want.
    blacklist: Option<Rc<RefCell<Blacklist>>>,
}

impl Grabber {
    /// Creates new instance of `Self`.
    pub fn new(request_sender: RequestSender) -> Self {
        Grabber {
            posts: vec![PostCollection::new("Single Posts", "", Vec::new())],
            request_sender,
            blacklist: None,
        }
    }

    /// Sets the blacklist.
    pub fn set_blacklist(&mut self, blacklist: Rc<RefCell<Blacklist>>) {
        if !blacklist.borrow_mut().is_empty() {
            self.blacklist = Some(blacklist);
        }
    }

    /// If the user supplies login information, this will grabbed the favorites from there account.
    pub fn grab_favorites(&mut self) {
        let login = Login::load().unwrap_or_else(|e| {
			error!("Unable to load `login.json`. Error: {}", e);
			warn!("The program will use default values, but it is highly recommended to check your login.json file to ensure that everything is correct.");
			Login::default()
		});
        if !login.username.is_empty() && login.download_favorites {
            let tag_str = format!("fav:{}", login.username);
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
    pub fn grab_posts_by_tags(&mut self, groups: &[Group]) {
        for group in groups {
            for tag in &group.tags {
                match tag.tag_type {
                    TagType::Pool => {
                        let entry: PoolEntry = self
                            .request_sender
                            .get_entry_from_appended_id(&tag.name, "pool");
                        let name = &entry.name;
                        let posts = self.special_search(&format!("pool:{}", entry.id));
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
                    TagType::Set => {
                        let entry: SetEntry = self
                            .request_sender
                            .get_entry_from_appended_id(&tag.name, "set");
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
                    TagType::Post => {
                        let entry: PostEntry = self
                            .request_sender
                            .get_entry_from_appended_id(&tag.name, "single");
                        let id = entry.id;
                        self.posts
                            .first_mut()
                            .unwrap()
                            .posts
                            .push(GrabbedPost::from(entry));

                        info!(
                            "Post with ID {} grabbed!",
                            console::style(format!("\"{}\"", id)).color256(39).italic()
                        );
                    }
                    TagType::General | TagType::Artist => {
                        let posts = self.get_posts_from_tag(tag);
                        self.posts.push(PostCollection::new(
                            &tag.name,
                            "General Searches",
                            GrabbedPost::entry_to_vec(posts),
                        ));
                        info!(
                            "{} grabbed!",
                            console::style(format!("\"{}\"", tag.name))
                                .color256(39)
                                .italic()
                        );
                    }
                    TagType::None => unreachable!(),
                };
            }
        }
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&self, tag: &Tag) -> Vec<PostEntry> {
        match tag.search_type {
            TagCategory::General => self.general_search(&tag.name),
            TagCategory::Special => self.special_search(&tag.name),
            TagCategory::None => unreachable!(),
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(&self, searching_tag: &str) -> Vec<PostEntry> {
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

    /// Scans through array of posts and removes any that violets the blacklist.
    fn filter_posts_with_blacklist(&self, posts: &mut Vec<PostEntry>) {
        if self.request_sender.is_authenticated() {
            if let Some(ref blacklist) = self.blacklist {
                blacklist.borrow_mut().filter_posts(posts);
            }
        }
    }

    /// Removes invalid posts, this is dependant on if the file url is null or if the post was deleted.
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

        match invalid_posts.cmp(&1) {
            Ordering::Less => {}
            Ordering::Equal => {
                trace!(
                    "A post was filtered for being invalid (due to the user not being logged in)"
                );
                info!("A post was filtered by e621...");
            }
            Ordering::Greater => {
                trace!("{} posts were filtered for being invalid (due to the user not being logged in)", invalid_posts);
                info!(
                    "{} posts had to be filtered by e621/e926...",
                    console::style(invalid_posts).cyan().italic(),
                );
            }
        }
    }
}
