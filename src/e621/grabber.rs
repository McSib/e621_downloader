extern crate failure;
extern crate reqwest;

use crate::e621::blacklist::Blacklist;
use crate::e621::io::tag::{Group, Parsed, Tag};
use crate::e621::io::Login;
use crate::e621::sender::{PoolEntry, PostEntry, RequestSender, SetEntry};
use failure::Error;
use reqwest::Url;

pub trait ToVec<T> {
    fn entry_to_vec(vec: Vec<PostEntry>) -> Vec<T>;
}

pub struct GrabbedPost {
    pub file_url: String,
    pub file_name: String,
    pub file_size: i64,
}

impl From<PostEntry> for GrabbedPost {
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

impl ToVec<GrabbedPost> for GrabbedPost {
    fn entry_to_vec(vec: Vec<PostEntry>) -> Vec<GrabbedPost> {
        let mut temp_vec = Vec::with_capacity(vec.len());
        for post in vec {
            temp_vec.push(GrabbedPost::from(post));
        }
        temp_vec
    }
}

pub struct Category(String);

impl ToString for Category {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

pub struct PostSet {
    pub set_name: String,
    pub category: Category,
    pub posts: Vec<GrabbedPost>,
}

impl PostSet {
    pub fn new(set_name: &str, category: &str, posts: Vec<GrabbedPost>) -> Self {
        PostSet {
            set_name: set_name.to_string(),
            category: Category(category.to_string()),
            posts,
        }
    }
}

/// Grabs all posts under a set of searching tags.
pub struct Grabber {
    /// All grabbed posts
    pub grabbed_posts: Vec<PostSet>,
    pub grabbed_single_posts: PostSet,
    request_sender: RequestSender,
    /// Blacklist used to throwaway posts that contain tags the user may not want
    blacklist: Vec<String>,
}

impl Grabber {
    /// Creates new instance of `Self`.
    pub fn new(request_sender: RequestSender, blacklist: Vec<String>) -> Self {
        Grabber {
            grabbed_posts: Vec::new(),
            grabbed_single_posts: PostSet::new("Single Posts", "", Vec::new()),
            request_sender,
            blacklist,
        }
    }

    /// Gets posts on creation using `tags` and searching with `urls`.
    /// Also modifies the `config` when searching general tags.
    pub fn from_tags(
        groups: &[Group],
        request_sender: RequestSender,
        blacklist: Vec<&str>,
    ) -> Result<Grabber, Error> {
        let mut grabber = Grabber::new(
            request_sender,
            blacklist.iter().map(|e| e.to_string()).collect(),
        );
        grabber.grab_favorites()?;
        grabber.grab_tags(groups)?;
        Ok(grabber)
    }

    pub fn grab_favorites(&mut self) -> Result<(), Error> {
        let login = Login::load()?;
        if !login.username.is_empty() && login.download_favorites {
            let tag_str = format!("fav:{}", login.username);
            let posts = self.custom_search(tag_str.as_str())?;
            self.grabbed_posts.push(PostSet::new(
                &tag_str,
                "Favorites",
                GrabbedPost::entry_to_vec(posts),
            ));
            println!("\"{}\" grabbed!", tag_str);
        }

        Ok(())
    }

    /// Iterates through tags and perform searches for each, grabbing them and storing them in `self.grabbed_tags`.
    pub fn grab_tags(&mut self, groups: &[Group]) -> Result<(), Error> {
        for group in groups {
            for tag in &group.tags {
                match tag {
                    Parsed::Pool(id) => {
                        let entry: PoolEntry = self.request_sender.get_entry_from_id(id, "pool")?;
                        self.grabbed_posts.push(PostSet::new(
                            &entry.name,
                            "Pools",
                            GrabbedPost::entry_to_vec(entry.posts),
                        ));

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Set(id) => {
                        let entry: SetEntry = self.request_sender.get_entry_from_id(id, "set")?;
                        self.grabbed_posts.push(self.set_to_named_entry(&entry)?);

                        println!("\"{}\" grabbed!", entry.name);
                    }
                    Parsed::Post(id) => {
                        let entry: PostEntry =
                            self.request_sender.get_entry_from_id(id, "single")?;
                        let id = entry.id;
                        self.grabbed_single_posts
                            .posts
                            .push(GrabbedPost::from(entry));

                        println!("Post with ID \"{}\" grabbed!", id);
                    }
                    Parsed::General(tag) => {
                        let tag_str = match tag {
                            Tag::General(tag_str) | Tag::Special(tag_str) => tag_str.clone(),
                            Tag::None => String::new(),
                        };
                        let posts = self.get_posts_from_tag(tag)?;
                        self.grabbed_posts.push(PostSet::new(
                            &tag_str,
                            "General Searches",
                            GrabbedPost::entry_to_vec(posts),
                        ));
                        println!("\"{}\" grabbed!", tag_str);
                    }
                };
            }
        }

        Ok(())
    }

    /// Grabs posts from general tag.
    fn get_posts_from_tag(&mut self, tag: &Tag) -> Result<Vec<PostEntry>, Error> {
        match tag {
            Tag::General(ref tag_search) => Ok(self.general_search(tag_search)?),
            Tag::Special(ref tag_search) => Ok(self.special_search(&mut tag_search.clone())?),
            Tag::None => bail!(format_err!("The tag is none!")),
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

            if !self.blacklist.is_empty() {
                let blacklist = Blacklist::new(&self.blacklist);
                blacklist.filter_posts(&mut searched_posts);
            }

            posts.append(&mut searched_posts);
        }

        Ok(posts)
    }

    /// Performs a special search that grabs from a date up to the current day.
    fn special_search(&mut self, searching_tag: &mut String) -> Result<Vec<PostEntry>, Error> {
        let mut page: u16 = 1;
        let mut posts: Vec<PostEntry> = vec![];
        loop {
            let mut searched_posts: Vec<PostEntry> =
                self.request_sender.bulk_search(searching_tag, page)?;
            if searched_posts.is_empty() {
                break;
            }

            if !self.blacklist.is_empty() {
                let blacklist = Blacklist::new(self.blacklist.as_slice());
                blacklist.filter_posts(&mut searched_posts);
            }

            posts.append(&mut searched_posts);
            page += 1;
        }

        Ok(posts)
    }

    /// Converts `SetEntry` to `NamedPost`.
    fn set_to_named_entry(&self, set: &SetEntry) -> Result<PostSet, Error> {
        let posts: Vec<PostEntry> = self.custom_search(&format!("set:{}", set.short_name))?;
        Ok(PostSet::new(
            &set.name,
            "Sets",
            GrabbedPost::entry_to_vec(posts),
        ))
    }

    fn custom_search(&self, tag: &str) -> Result<Vec<PostEntry>, Error> {
        let mut posts = vec![];
        let mut page = 1;
        loop {
            let mut set_posts: Vec<PostEntry> = self.request_sender.bulk_search(tag, page)?;
            if set_posts.is_empty() {
                break;
            }

            posts.append(&mut set_posts);
            page += 1;
        }

        Ok(posts)
    }
}
