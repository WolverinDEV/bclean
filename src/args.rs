use std::{
    error::Error,
    path::PathBuf,
};

use bclean::{
    sweeper::{
        CMakeSweeper,
        NodeSweeper,
        RustSweeper,
    },
    Sweeper,
};
use clap::{
    Parser,
    ValueEnum,
};

#[derive(Clone, ValueEnum, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ArgSweeper {
    CMake,
    Node,
    Rust,
}

impl ArgSweeper {
    pub fn parse_args(
        value: &str,
    ) -> Result<(ArgSweeper, Option<String>), Box<dyn Error + Send + Sync>> {
        let result = if let Some((sweeper, options)) = value.split_once("=") {
            (
                ArgSweeper::from_str(sweeper, true)?,
                Some(options.to_string()),
            )
        } else {
            (ArgSweeper::from_str(value, true)?, None)
        };

        Ok(result)
    }

    pub fn default_configuration() -> Vec<(ArgSweeper, Option<String>)> {
        vec![
            (ArgSweeper::Node, None),
            (ArgSweeper::Rust, None),
            (ArgSweeper::CMake, None),
        ]
    }

    pub fn create_from_options(&self, _options: Option<&str>) -> anyhow::Result<Box<dyn Sweeper>> {
        let result: Box<dyn Sweeper> = match self {
            Self::CMake => Box::new(CMakeSweeper::new()),
            Self::Node => Box::new(NodeSweeper::new()),
            Self::Rust => Box::new(RustSweeper::new()),
        };

        Ok(result)
    }
}

/// Automate the cleanup of left over build files
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Specify the root directory where bclean should search for sweepable targets.
    /// Note: This can be a relative path.
    #[arg(short, long, verbatim_doc_comment)]
    pub root: Option<PathBuf>,

    /// Display the log in the terminal as a split screen.
    #[arg(long)]
    pub ui_logger: bool,

    /// Do not actually sweep anything. Just simulate it.
    #[arg(short, long)]
    pub dry_run: bool,

    /// Specify a list of sweeper which should be activated.
    /// Additionally you can specify sweeper individual arguments.
    ///
    /// Available sweeper:
    /// - c-make
    /// - rust
    /// - node
    ///
    /// Example:
    /// -s rust -s "cmake=dirname=cmake,build,dist"
    #[arg(value_parser = ArgSweeper::parse_args, short, long, verbatim_doc_comment)]
    pub sweeper: Vec<(ArgSweeper, Option<String>)>,

    /// Do not apply the default sweeper
    #[arg(long)]
    pub sweeper_no_defaults: bool,
}
