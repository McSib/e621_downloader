extern crate failure;
extern crate reqwest;
extern crate serde_json;

use failure::Error;
use reqwest::Url;
use serde_json::Value;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::tag::{Group, Tag, TagCategory, TagType};
use crate::e621::io::Login;
use crate::e621::sender::{PoolEntry, PostEntry, RequestSender, SetEntry};

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
}

impl From<PostEntry> for GrabbedPost {
    /// Converts `PostEntry` to `Self`.
    fn from(post: PostEntry) -> Self {
        GrabbedPost {
            file_url: post.file_url.clone(),
            file_name: Url::parse(post.file_url.as_str())
                .unwrap()
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .to_string(),
            file_size: post.file_size.unwrap_or_default(),
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
    pub fn from_tags(groups: &[Group], request_sender: RequestSender) -> Result<Grabber, Error> {
        let mut grabber = Grabber::new(request_sender);
        grabber.grab_blacklist()?;
        grabber.grab_favorites()?;
        grabber.grab_tags(groups)?;
        Ok(grabber)
    }

    /// If login information is supplied, the connector will log into the supplied account and obtain it's blacklist.
    /// This should be the only time the connector ever logs in.
    pub fn grab_blacklist(&mut self) -> Result<(), Error> {
        let login = Login::load()?;
        if !login.is_empty() {
            let json: Value = self.request_sender.get_blacklist(&login)?;
            let blacklist_string = json["blacklist"]
                .to_string()
                .trim_matches('\"')
                .replace("\\n", "\n");
            let blacklist_entries: Vec<String> =
                blacklist_string.lines().map(|e| (*e).to_string()).collect();
            self.blacklist = if !blacklist_entries.is_empty() {
                Some(Blacklist::new(&blacklist_entries))
            } else {
                None
            };
        }

        Ok(())
    }

    /// If the user supplies login information, this will grabbed the favorites from there account.
    pub fn grab_favorites(&mut self) -> Result<(), Error> {
        let login = Login::load()?;
        if !login.username.is_empty() && login.download_favorites {
            let tag_str = format!("fav:{}", login.username);
            let posts = self.special_search(tag_str.as_str())?;
            self.grabbed_posts.push(PostSet::new(
                &tag_str,
                "Favorites",
                GrabbedPost::entry_to_vec(posts),
            ));
            println!("\"{}\" grabbed!", tag_str);
        }

        Ok(())
    }

    /// Iterates through tags and perform searches for each, grabbing them and storing them for later download.
    pub fn grab_tags(&mut self, groups: &[Group]) -> Result<(), Error> {
        for group in groups {
            for tag in &group.tags {
                match tag.tag_type {
                    TagType::Pool => {
                        let entry: PoolEntry =
                            self.request_sender.get_entry_from_id(&tag.raw, "pool")?;
                        self.grabbed_posts.push(PostSet::new(
                            &entry.name,
                            "Pools",
                            GrabbedPost::entry_to_vec(entry.posts),
                        ));

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    TagType::Set => {
                        let entry: SetEntry =
                            self.request_sender.get_entry_from_id(&tag.raw, "set")?;
                        self.grabbed_posts.push(self.set_to_named_entry(&entry)?);

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    TagType::Post => {
                        let entry: PostEntry =
                            self.request_sender.get_entry_from_id(&tag.raw, "single")?;
                        let id = entry.id;
                        self.grabbed_single_posts
                            .posts
                            .push(GrabbedPost::from(entry));

                        println!("Post with ID \"{}\" grabbed!", id);
                    }
                    TagType::General | TagType::Artist => {
                        let posts = self.get_posts_from_tag(tag)?;
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

        Ok(())
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&mut self, tag: &Tag) -> Result<Vec<PostEntry>, Error> {
        match tag.search_type {
            TagCategory::General => Ok(self.general_search(&tag.raw)?),
            TagCategory::Special => Ok(self.special_search(&tag.raw)?),
            TagCategory::None => unreachable!(),
        }
    }

    /// Performs a general search where it grabs only five pages of posts.
    fn general_search(&mut self, searching_tag: &str) -> Result<Vec<PostEntry>, Error> {
        let limit: u16 = 5;
        let mut posts: Vec<PostEntry> = Vec::with_capacity(320 * limit as usize);
        for page in 1..limit {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page)?;
            if searched_posts.is_empty() {
                break;
            }

            if let Some(ref e) = self.blacklist {
                e.filter_posts(&mut searched_posts);
            }

            posts.append(&mut searched_posts);
        }

        Ok(posts)
    }

    /// Performs a special search that grabs all posts tied to the searching tag.
    fn special_search(&self, searching_tag: &str) -> Result<Vec<PostEntry>, Error> {
        let mut page: u16 = 1;
        let mut posts: Vec<PostEntry> = vec![];
        loop {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page)?;
            if searched_posts.is_empty() {
                break;
            }

            if let Some(ref e) = self.blacklist {
                e.filter_posts(&mut searched_posts);
            }

            posts.append(&mut searched_posts);
            page += 1;
        }

        Ok(posts)
    }

    /// Converts `SetEntry` to `PostSet`.
    fn set_to_named_entry(&self, set: &SetEntry) -> Result<PostSet, Error> {
        Ok(PostSet::new(
            &set.name,
            "Sets",
            GrabbedPost::entry_to_vec(self.special_search(&format!("set:{}", set.short_name))?),
        ))
    }
}
