extern crate chrono;
extern crate console;
extern crate pbr;
extern crate reqwest;
extern crate serde;

use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::{Read, stdin, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use chrono::{Date, Local};
use console::Term;
use pbr::ProgressBar;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};

use crate::e621::io::Config;
use crate::e621::io::tag::{Tag};

pub mod io;

static USER_AGENT_PROJECT_NAME: &'static str = "e621_downloader/0.0.1 (by McSib on e621)";
static DEFAULT_DATE: &'static str = "2006-01-01";

/// Time the post was created.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatedAt {
    pub json_class: String,
    pub s: i64,
    pub n: i64,
}

/// Post from e621 or E926.
/// 
/// # Important
/// If the post that is loaded happens to be deleted when loaded, these properties will not be usable:
/// `source`, `sources`, `md5`, `file_size`, `file_ext`, `preview_width`, `preview_height`, `sample_url`, `sample_width`, `sample_height`, `has_children`, `children`.
#[derive(Serialize, Deserialize, Debug)]
pub struct Post {
    /// The ID of the post
    pub id: i64,
    /// Tags from the post
    pub tags: String,
    /// Tags that are locked by the admins
    pub locked_tags: Option<String>,
    /// Description of the post
    pub description: String,
    /// When the post was uploaded
    pub created_at: CreatedAt,
    /// User ID of the user who uploaded the post
    pub creator_id: Option<i64>,
    /// Username of the user who uploaded the post
    pub author: String,
    /// The amount of changes that the post went through since uploaded
    pub change: i64,
    /// The main source of the work (use `sources` instead when using all source listed on post)
    pub source: Option<String>,
    /// How many upvoted or downvoted the post
    pub score: i64,
    /// How many favorites the post has
    pub fav_count: i64,
    /// The MD5 certification of the post
    pub md5: Option<String>,
    /// Size of the source file
    pub file_size: Option<i64>,
    /// URL of the source file
    pub file_url: String,
    /// Extension of the source file (png, jpg, webm, gif, etc)
    pub file_ext: Option<String>,
    /// URL for the preview file
    pub preview_url: String,
    /// Width of the preview file
    pub preview_width: Option<i64>,
    /// Height of the preview file
    pub preview_height: Option<i64>,
    /// URL for the sample file
    pub sample_url: Option<String>,
    /// Width of the sample file
    pub sample_width: Option<i64>,
    /// Height of the sample file
    pub sample_height: Option<i64>,
    /// Rating of the post (safe, questionable, explicit), this will be "s", "q", "e"
    pub rating: String,
    /// Post status, one of: active, flagged, pending, deleted
    pub status: String,
    /// Width of image
    pub width: i64,
    /// Height of image
    pub height: i64,
    /// If the post has comments
    pub has_comments: bool,
    /// If the post has notes
    pub has_notes: bool,
    /// If the post has children
    pub has_children: Option<bool>,
    /// All of the children attached to post
    pub children: Option<String>,
    /// If this post is a child, this will be the parent post's ID
    pub parent_id: Option<i64>,
    /// The artist or artists that drew this image
    pub artist: Vec<String>,
    /// All the sources for the work
    pub sources: Option<Vec<String>>,
}

#[derive(Debug)]
/// Stores all posts from searched parsed tag.
struct TagPosts {
    pub searching_tag: String,
    pub posts: Vec<Post>,
}

#[derive(Debug)]
/// Contains posts tied to a specific group.
struct GroupPosts {
    /// Name of group
    pub group_name: String,
    /// All posts in group
    pub tag_posts: Vec<TagPosts>,
}

/// Basic web connector for e621.
pub struct EsixWebConnector<'a> {
    /// Url used for connecting and downloading images
    url: String,
    /// Whether the site is the safe version or note. If true, it will force connection to E926 instead of E621
    safe: bool,
    /// Configuration data used for downloading images and tag searches
    config: &'a mut Config,
    /// Web client to connect and download images.
    client: Client,
    /// All posts grabbed from e621 search.
    groups: Vec<GroupPosts>,
}

impl<'a> EsixWebConnector<'a> {
    /// Creates new EWeb object for connecting and downloading images.
    ///
    /// # Example
    ///
    /// ```
    /// let connector = EWeb::new();
    /// ```
    pub fn new(config: &mut Config) -> EsixWebConnector {
        let connector = EsixWebConnector {
            url: "https://e621.net/post/index.json".to_string(),
            safe: false,
            config,
            client: Client::new(),
            groups: Vec::new(),
        };

        if connector.config.part_used_as_name != "md5" && connector.config.part_used_as_name != "id" {
            println!("Config `part_used_as_name` is set incorrectly!");
            println!("This will auto set to `md5` for this image, but you should fix the config when done with the program.");
        }

        connector
    }

    /// Sets the site into safe mode so no NSFW images popup in the course of downloading.
    ///
    /// # Example
    ///
    /// ```
    /// let connector = EWeb::new();
    /// connector.set_safe();
    /// ```
    pub fn set_safe(&mut self) {
        self.safe = true;
        self.update_to_safe_url();
    }

    /// Checks if the user wants to use safe mode.
    pub fn check_for_safe_mode(&mut self) -> Result<(), Box<Error>> {
        let mut response = String::new();
        loop {
            println!("Do you want to use safe mode (e926)? (Y/N)");
            response.clear();
            stdin().read_line(&mut response)?;
            match response.to_lowercase().trim() {
                "y" => {
                    self.set_safe();
                    break;
                },
                "n" => break,
                _ => {
                    let term = Term::stdout();
                    println!("Input invalid!");
                    sleep(Duration::from_millis(1000));
                    term.clear_screen()?;
                }
            }
        }

        Ok(())
    }

//    /// Gets posts with tags supplied and iterates through pages until no more posts available.
//    pub fn get_posts(&mut self, groups: &Vec<Group>) -> Result<(), Box<Error>> {
//        for group in groups {
//            if group.tags.is_empty() {
//                continue;
//            }
//
//            let mut tag_posts: Vec<TagPosts> = vec![];
//            for tag in &group.tags {
//                println!("Grabbing posts tagged: {}", tag.value);
//                self.update_tag_date(tag);
//
//                let mut page = 1;
//                let mut json: Vec<Post>;
//                let mut posts: Vec<Post> = vec![];
//                loop {
//                    json = self.client.get(&self.url)
//                               .header(USER_AGENT, USER_AGENT_PROJECT_NAME)
//                               .query(&[("tags", format!("{} date:>={}", tag.value, self.config.last_run[&tag.value])),
//                                   ("page", format!("{}", page)),
//                                   ("limit", String::from("320"))])
//                               .send()
//                               .expect("Unable to make connection to e621!")
//                               .json::<Vec<Post>>()?;
//                    if json.len() <= 0 {
//                        break;
//                    }
//
//                    posts.append(&mut json);
//                    page += 1;
//                }
//
//                tag_posts.push(TagPosts {
//                    searching_tag: tag.value.clone(),
//                    posts
//                });
//            }
//
//            self.groups.push(GroupPosts {
//                group_name: group.group_name.clone(),
//                tag_posts
//            });
//
//            println!("{:#?}", self.groups);
//        }
//
//        Ok(())
//    }

    /// Updates config `last_run` to hold new date.
    fn update_tag_date(&mut self, tag: &Tag) {
        let date: Date<Local> = Local::today();
        self.config.last_run.entry(tag.value.clone())
            .and_modify(|e| *e = date.format("%Y-%m-%d").to_string())
            .or_insert(DEFAULT_DATE.to_string());
    }

    /// Downloads images from collected posts.
    ///
    /// # Warning
    /// Since request are sent through a single thread, the progress bar may slow down progress.
    /// This, along with other issues, is causing the program to take longer to download images (about 20+ minutes for 2000+ images).
    pub fn download_posts(&self) -> Result<(), Box<Error>> {
        // TODO: Program different treatment of groups.
        for tag_post in &self.groups[0].tag_posts {
            let mut progress_bar = ProgressBar::new(tag_post.posts.len() as u64);
            progress_bar.message(format!("{} ", tag_post.searching_tag).as_str());

            for post in &tag_post.posts {
                let name = self.get_name_for_image(post);
                let mut image: Vec<u8> = Vec::new();
                self.client.get(post.file_url.as_str())
                    .header(USER_AGENT, USER_AGENT_PROJECT_NAME)
                    .send()?
                    .read_to_end(&mut image)?;
                self.save_image(&name, &image, post.file_ext.as_ref().unwrap(), &tag_post.searching_tag.clone())?;
                progress_bar.inc();
            }

            progress_bar.finish_println("Posts downloaded!");
        }

        Ok(())
    }

    /// Saves image to directory described in config.
    fn save_image(&self, name: &String, source: &Vec<u8>, image_type: &String, tag: &String) -> Result<(), Box<Error>> {
        let download_dest = format!("{}{}", self.config.download_directory, tag);
        if !Path::new(download_dest.as_str()).exists() {
            create_dir_all(Path::new(download_dest.as_str()))?;
        }

        let mut file: File;
        if self.config.create_directories {
            file = File::create(Path::new(format!("{}{}/{}.{}", self.config.download_directory, tag, name, image_type).as_str()))?;
            file.write(source)?;
        } else {
            file = File::create(Path::new(format!("{}{}.{}", self.config.download_directory, name, image_type).as_str()))?;
            file.write(source)?;
        }

        Ok(())
    }

    /// Gets name that will be used for saving image.
    fn get_name_for_image(&self, post: &Post) -> String {
        match self.config.part_used_as_name.as_str() {
            "md5" => post.md5.as_ref().unwrap().clone(),
            "id" => format!("{}", post.id),
            _ => post.md5.as_ref().unwrap().clone(),
        }
    }

    /// Updates the url for safe mode.
    fn update_to_safe_url(&mut self) {
        self.url = self.url.replace("e621", "e926");
    }
}
