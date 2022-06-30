#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

use std::env::consts::{
    ARCH,
    DLL_EXTENSION,
    DLL_PREFIX,
    DLL_SUFFIX,
    EXE_EXTENSION,
    EXE_SUFFIX,
    FAMILY,
    OS,
};
use std::fs::File;

use failure::Error;
use simplelog::{
    ColorChoice,
    CombinedLogger,
    Config,
    ConfigBuilder,
    LevelFilter,
    TermLogger,
    TerminalMode,
    WriteLogger,
};
use terminal_menu::{
    back_button,
    button,
    label,
    list_with_default_value,
    menu,
    mut_menu,
    run,
    scroll_with_default_value,
    string,
    submenu,
    TerminalMenu,
};

use crate::e621::io::Login;
use crate::program::Program;

mod e621;
mod program;

/// Logs important information about the system being used.
/// This is useful for debugging purposes.
/// This function is called automatically by the `main` function.
///
/// # Example
/// ```rust
/// log_system_information();
/// ```
///
/// # Output
/// ```text
/// OS: linux
/// ARCH: x86_64
/// FAMILY: unix
/// DLL_EXTENSION: .so
/// DLL_PREFIX: lib
/// DLL_SUFFIX: .so
/// EXE_EXTENSION: .so
/// EXE_SUFFIX: .so
/// ```
fn log_system_information() {
    trace!("Printing system information out into log for debug purposes...");
    trace!("ARCH:           \"{}\"", ARCH);
    trace!("DLL_EXTENSION:  \"{}\"", DLL_EXTENSION);
    trace!("DLL_PREFIX:     \"{}\"", DLL_PREFIX);
    trace!("DLL_SUFFIX:     \"{}\"", DLL_SUFFIX);
    trace!("EXE_EXTENSION:  \"{}\"", EXE_EXTENSION);
    trace!("EXE_SUFFIX:     \"{}\"", EXE_SUFFIX);
    trace!("FAMILY:         \"{}\"", FAMILY);
    trace!("OS:             \"{}\"", OS);
}

fn main() -> Result<(), Error> {
    let mut config = ConfigBuilder::new();
    config.add_filter_allow_str("e621_downloader");
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::max(),
            config.build(),
            File::create("e621_downloader.log").unwrap(),
        ),
    ])
    .unwrap();

    log_system_information();

    if !e621::io::Config::config_exists() {
        e621::io::Config::create_config().unwrap();
    }

    let mut config = e621::io::Config::get_config().unwrap();
    let mut login = Login::load().unwrap();

    let menu = menu(vec![
        label("e621_downloader"),
        button("Run Downloader"),
        button("Edit Tags"),
        submenu(
            "Settings",
            vec![
                submenu(
                    "Config Settings",
                    vec![
                        string("Download Directory", config.download_directory(), false),
                        scroll_with_default_value(
                            "Naming Convention",
                            vec!["md5", "id"],
                            match config.naming_convention() {
                                "md5" => 0,
                                "id" => 1,
                                _ => 0,
                            },
                        ),
                        back_button("Back"),
                    ],
                ),
                submenu(
                    "Login Settings",
                    vec![
                        string("Username", login.username(), true),
                        string("API Key", login.api_key(), true),
                        list_with_default_value(
                            "Download Favorites",
                            vec!["True", "False"],
                            match login.download_favorites() {
                                true => 0,
                                false => 1,
                            },
                        ),
                        back_button("Back"),
                    ],
                ),
                back_button("Back"),
            ],
        ),
        back_button("Exit"),
    ]);

    run(&menu);

    update_config(&mut config, &menu);
    update_login(&mut login, &menu);

    let edit_tags = mut_menu(&menu).selected_item_name() == "Edit Tags";
    if edit_tags {
        edit_tags_menu();
    }

    let start = mut_menu(&menu).selected_item_name() == "Run Downloader";
    if start {
        let program = Program::new();
        program.run().unwrap();
    }

    Ok(())
}

fn update_config(config: &mut e621::io::Config, menu: &TerminalMenu) {
    let mut mut_menu = mut_menu(menu);
    let mut settings_guard = mut_menu.get_submenu("Settings");
    let config_settings_guard = settings_guard.get_submenu("Config Settings");
    let download_directory = config_settings_guard.selection_value("Download Directory");
    let naming_convention = config_settings_guard.selection_value("Naming Convention");

    config.set_download_directory(download_directory.to_string());
    config.set_naming_convention(naming_convention.to_string());
    config.save_config();
}

fn update_login(login: &mut Login, menu: &TerminalMenu) {
    let mut mut_menu = mut_menu(menu);
    let mut settings_guard = mut_menu.get_submenu("Settings");
    let login_settings_guard = settings_guard.get_submenu("Login Settings");
    let username = login_settings_guard.selection_value("Username");
    let api_key = login_settings_guard.selection_value("API Key");
    let download_favorites = login_settings_guard.selection_value("Download Favorites");

    login.set_username(username.to_string());
    login.set_api_key(api_key.to_string());
    login.set_download_favorites(match download_favorites {
        "True" => true,
        "False" => false,
        _ => false,
    });

    login.save_login();
}

fn edit_tags_menu() {
    // Editing tags will be different from what it used to be.
    // There will be five categories: Artist, pools, sets, single posts, and general.
    // There will be a menu with six buttons, five of which will be for the five categories. The last button will be for the back button.
    // Whenever the user clicks on a button, the menu will close and then a new menu will open, listing all tags for the category that was clicked on,
    // with an option to add a new tag or to go back. If the user adds a new tag, it will validate that the tag is not already in the list and that the
    // tag exists on e621. If the tag is valid, it will be added to the list. If the tag is not valid, it will not be added to the list.
    // It will also tell the user if the tag is invalid or already in the list.
    // let mut tag_menu = menu(
    //     vec![
    //         label("Edit Tags:"),
    //         button("Artist"),
    //         button("Pools"),
    //         button("Sets"),
    //         button("Single Posts"),
    //         button("General"),
    //         back_button("Back"),
    //     ],
    // );

    // run(&tag_menu);

    // let mut mut_menu = mut_menu(&tag_menu);
    // TODO: Implement a way to edit tags.
    // if mut_menu.selected_item_name() == "Artist" {
    //     edit_tags_artist();
    // } else if mut_menu.selected_item_name() == "Pools" {
    //     edit_tags_pools();
    // } else if mut_menu.selected_item_name() == "Sets" {
    //     edit_tags_sets();
    // } else if mut_menu.selected_item_name() == "Single Posts" {
    //     edit_tags_single_posts();
    // } else if mut_menu.selected_item_name() == "General" {
    //     edit_tags_general();
    // }
}
