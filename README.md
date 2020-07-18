# E621 Downloader

E621 Downloader is a program made entirely in Rust, a low-level language with similar performance to C, with the intention of being a cross platform application made for easy downloading and archiving of images from tags.

### Goal

The goal of this application is to keep up-to-date with your favorite artist, download pools, and grab images from normal, everyday tags.

## About E621/E926

E621 is a mature image board replacement for the image board Sidechan. A general audience image board, e926 (formerly e961) complements this site. E621 runs off of the Ouroboros platform, a danbooru styled software specifically designed for the site.

E621 has over 1,680,900+ images and videos hosted on its platform, with this overwhelming number, it can be hard to find images you enjoy or artist you like.

## Todo list for future updates

This list is constantly updated and checked for new features and addons that need to be implemented in the future. My hope with all this work is to have a downloader that will last a long time for the next forseable future, as there are many downloaders like this that have either ceased developement, or found to be too hard or confusing to operate for the average person.

 - [x] Add the ability to not create directories for images.
 - [x] Let the user sign in and use their blacklist.
 - [x] Let the user be able to download their favorites.
 - [x] Bring more general improvement and optimization to the code for faster runtime.
 - [x] Allow the program to work with aliases (the validator searches for tags in the most literal way, so aliases won't slide right now).

# FAQ

### Why does the program only grab only 1,280 posts with certain tags?

When a tag passes the limit of **1,500** posts, it is **considered too large a collection for the software to download**. The program will opt to download only 5 pages worth of posts to compensate for this hard limit. The pages use the **highest post limit** the e621/e926 servers will allow, which is **320 posts per page**. In total, it will grab **1,280 posts as its maximum**.

Something to keep a note of, depending on the type of tag, the program will either ignore or use this limit. This is handled low-level by categorizing the tag into two sections: **General** and **Special**.

General will force the program to use the 1,280 post limit. The tags that register under this flag are as such: **General** (this is basic tags, such as `fur`, `smiling`, `open_mouth`), **Copyright** (any form of copyrighted media should always be considered too large to download in full), **Species** (since species are very close to general in terms of number of posts they can hold, it will be treated as such), and **Character in special cases** (when a character has greater than 1,500 posts tied to them, it will be considered a General tag to avoid longer wait times while downloading).

Tags that register under the Special flag are as such: **Artist** (generally, if you are grabbing an artist's work directly, you plan to grab all their work for archiving purposes. Thus, it will always be considered Special), and **Character** (if the amount of posts tied to the character is below 1,500, it will be considered a Special tag and the program will download _all_ posts with the character in it).

This system is more complex than what I have explained so far, but in a basic sense, this is how the downloading function works with tags directly. These checks and grabs happen with a tight-knit relationship that is carried with the parser and the downloader. The parser will help grab the number of posts and also categorize the tags to their correct spots while the downloader focuses on using these tag types to grab and download their posts correctly.

Hopefully, this explains how and why the limit is there.

### Compiling on GNU/Linux

## Debian 10 & Derivatives

To successfully build on a Debian 10-based GNU/Linux system, first install rustc and cargo.
At the time of writing, the versions included with Debian 10 work, which are cargo and rustc version 1.43.0.

`sudo apt install rustc cargo`

Then clone the git repository into your desired location.

`git clone https://github.com/McSib/e621_downloader`

After it's finished cloning, simply enter the directory and run the following:

`cargo build`

This could take quite a while depending on your CPU. This has only been tested on an amd64 CPU (an Intel Core 2 Duo), which took about fourty minutes.

### macOS Builds

Building on macOS remain untested. If you have access to this platform, please make a PR or file an issue for build instructions, potential errors, and fixes.
