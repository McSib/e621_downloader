#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

use std::{
    env::consts::{
        ARCH, DLL_EXTENSION, DLL_PREFIX, DLL_SUFFIX, EXE_EXTENSION, EXE_SUFFIX, FAMILY, OS,
    },
    fs::File,
};

use failure::Error;
use simplelog::{
    ColorChoice, CombinedLogger, Config, ConfigBuilder, LevelFilter, TermLogger, TerminalMode,
    WriteLogger,
};

use crate::program::Program;

mod e621;
mod program;

fn main() -> Result<(), Error> {
    initialize_logger();
    log_system_information();

    let program = Program::new();
    program.run()
}

fn initialize_logger() {
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
}

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
