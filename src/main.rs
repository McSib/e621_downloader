#[macro_use]
extern crate failure;
#[macro_use]
extern crate log;

use std::fs::File;

use failure::Error;
use simplelog::{
    ColorChoice, CombinedLogger, Config, ConfigBuilder, LevelFilter, TermLogger, TerminalMode,
    WriteLogger,
};

use crate::program::Program;

mod e621;
mod program;

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

    let program = Program::new();
    program.run()
}
