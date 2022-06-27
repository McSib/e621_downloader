# E621 Downloader
![Release](https://img.shields.io/github/release/McSib/e621_downloader.svg)
![Commits Since](https://img.shields.io/github/commits-since/McSib/e621_downloader/latest.svg)
![Stars](https://img.shields.io/github/stars/McSib/e621_downloader.svg)
![Watchers](https://img.shields.io/github/watchers/McSib/e621_downloader.svg)
![Forks](https://img.shields.io/github/forks/McSib/e621_downloader.svg)

![Maintained?](https://img.shields.io/badge/Maintained%3F-yes-green.svg)
![Lisence](https://img.shields.io/github/license/McSib/e621_downloader.svg)
![Downloads](https://img.shields.io/github/downloads/McSib/e621_downloader/total.svg)

![Issues Open](https://img.shields.io/github/issues/McSib/e621_downloader.svg)
![Issues Closed](https://img.shields.io/github/issues-closed/McSib/e621_downloader.svg)
![PR Open](https://img.shields.io/github/issues-pr/McSib/e621_downloader.svg)
![PR Closed](https://img.shields.io/github/issues-pr-closed/McSib/e621_downloader.svg)

[![Rust](https://github.com/McSib/e621_downloader/actions/workflows/rust.yml/badge.svg?branch=active)](https://github.com/McSib/e621_downloader/actions/workflows/rust.yml)

The `e621_downloader` is a low-level, close-to-hardware program meant to download a large number of images at a fast pace. It can handle bulk posts, single posts, sets, and pools via a custom easy-to-read language that I made.

Having tested this software extensively with downloading, I managed to download 10,000 posts (which was over 20+ GB) in just two hours, averaging around 20MB/s.

### Goal

The goal of this application is to keep up-to-date with your favorite artist, download pools, grab images from normal, everyday tags, while most of all, staying reliable.

## About E621/E926

E621 is a mature image board replacement for the image board Sidechan. A general audience image board, e926 (formerly e961) complements this site. E621 runs off of the Ouroboros platform, a danbooru styled software specifically designed for the site.

E621 has over 2,929,253+ images and videos hosted on its platform.

## Todo list for future updates

This list is constantly updated and checked for new features and addons that need to be implemented in the future. My hope with all this work is to have a downloader that will last a long time for the next forseable future, as there are many downloaders like this that have either ceased developement, or found to be too hard or confusing to operate for the average person.

 - [ ] Add a menu system with configuration editing, tag editing, and download configuration built in.
 - [ ] Transition the tag file language into Json to integrate easily with the menu system.
 - [ ] Update the code to be more sound, structured, and faster.

# Installation Guide (Windows)
1. If you are on Windows, simply visit this [link](https://rustup.rs) and install rust and cargo through the installer provided. _You will need GCC or MSVC in order to compile the project, so choose either 1 or 2 for this._
    1. To get GCC, I would recommend [this](https://winlibs.com) helpful little site which contains the most up to date versions of GCC. Note, however, that you will need to unzip this in a directory you make, and will have to link the bin folder in that directory to your `PATH` for it to work.
    2. For MSVC, just go to this [link](https://visualstudio.microsoft.com/downloads/?q=build+tools) and download the Visual Studio Build Tools (at the very bottom of the page), which will install all the needed binaries without the full Visual Studio IDE.

2. Now, once you've done that, you can either clone the GitHub project directly through Git, or download a zip of the latest version. You can download Git from [here](https://git-scm.com/downloads).
    - If you choose to use Git, find a proper directory you want the project in, and then type in `git clone https://github.com/McSib/e621_downloader.git` into a console and pressing Enter. This will clone the directory and prepare the project for you to modify or just compile.
3. No matter what option you chose, you want to open a terminal (CMD or Terminal) and go into the root directory of the project (where Cargo.toml and Cargo.lock are located). Inside this directory, type in `cargo build` or `cargo build --release`. If the program compiles and works, you're good to go.

# Installation Guide (Arch Linux)
1. For Arch Linux users, you will need a couple things installed in order to get the project up and running. The first thing you want to do is get the packages required (if you haven't). Run this command to download everything you need.

```
sudo pacman -S rust base-devel openssl git gdb
```

2. The next thing you will need to do is clone the git repository in a directory of your choosing.

```
git clone https://github.com/McSib/e621_downloader.git
```

3. From there, go into the newly cloned directory, and see if you can build by running `cargo build --release` or `cargo build`. If it compiled okay, then you are good to go.

4. **You can also now download a prebuilt binary of the program on the release page if you just want to use the program with little hassle.**

# Installation Guide (Debian)
1. This is very much like the Arch Linux setup with some minor tweaks to the package download command. Instead of the pacman command, enter: `sudo apt install gcc g++ gdb cargo libssl-dev git` and then follow step 2 from the Arch Linux installation forward.

# FAQ

### Why does the program only grab only 1,280 posts with certain tags?

When a tag passes the limit of **1,500** posts, it is **considered too large a collection for the software to download** as the size of all the files combined will not only put strain on the server, but on the program as well as the system it runs on. The program will opt to download only 5 pages worth of posts to compensate for this hard limit. The pages use the **highest post limit** the e621/e926 servers will allow, which is **320 posts per page**. In total, it will grab **1,280 posts as its maximum**.

Something to keep a note of, depending on the type of tag, the program will either ignore or use this limit. This is handled low-level by categorizing the tag into two sections: **General** and **Special**.

General will force the program to use the 1,280 post limit. The tags that register under this flag are as such: **General** (this is basic tags, such as `fur`, `smiling`, `open_mouth`), **Copyright** (any form of copyrighted media should always be considered too large to download in full), **Species** (since species are very close to general in terms of number of posts they can hold, it will be treated as such), and **Character in special cases** (when a character has greater than 1,500 posts tied to them, it will be considered a General tag to avoid longer wait times while downloading).

Tags that register under the Special flag are as such: **Artist** (generally, if you are grabbing an artist's work directly, you plan to grab all their work for archiving purposes. Thus, it will always be considered Special), and **Character** (if the amount of posts tied to the character is below 1,500, it will be considered a Special tag and the program will download _all_ posts with the character in it).

This system is more complex than what I have explained so far, but in a basic sense, this is how the downloading function works with tags directly. These checks and grabs happen with a tight-knit relationship that is carried with the parser and the downloader. The parser will help grab the number of posts and also categorize the tags to their correct spots while the downloader focuses on using these tag types to grab and download their posts correctly.

Hopefully, this explains how and why the limit is there.

# Notice for users using the new version (1.6.0 and newer)
If you are not logged into e621, a filter (almost like a global blacklist) is applied. This blacklist will nullify any posts that fall under its settings. So, if you notice images you're trying to download aren't showing up, log in and then download it, otherwise, this filter will continue blacklisting them.

# Notice for users using a VPN
I have had a recurring "bug" that has shown in my issues the last couple of months, and they tend to crop up right after a new release, so I am going to supply a new notice for those using VPNs to prevent this becoming issue spam. There are users who are experiencing crashes consistently when parsing, obtaining blacklist, or downloading. It is an issue that is consistent, and each person thus far have been using a VPN with no other noticeable cause linked. After a multitude of testing, I have concluded that users using VPNs will occasionally have either e621 directly or Cloudflare prompt for a captcha, or a test for whether you are a robot. Since my program does not support GUI, or no tangible way of handling that, it will crash immediately. I have looked for fixes to this issue and have yet to find anything. So, if you are using a VPN, be warned, this can happen. The current work around for this issue is switching locations in the VPN (if you have that feature) or disabling the VPN altogether (if you have that option). I understand it is annoying, and can be a pain, but this is all I can do until I come across a fix. Sorry for the inconvenience, and apologies if you are some of the users experiencing this issue.