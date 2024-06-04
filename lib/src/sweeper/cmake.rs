use std::path::Path;

use super::{
    Sweeper,
    SweeperError,
};
use crate::{
    target::{
        DirectoryTarget,
        SweepableTarget,
    },
    PathEx,
};

pub struct CMakeSweeper;

impl CMakeSweeper {
    pub fn new() -> Self {
        Self
    }
}

impl Sweeper for CMakeSweeper {
    fn name(&self) -> &str {
        "cmake"
    }

    fn identify_targets(&self, path: &Path) -> Result<Vec<Box<dyn SweepableTarget>>, SweeperError> {
        if !path.is_dir() {
            return Ok(vec![]);
        }

        /* TODO: Properly check the CMAKE cache file onl remove build files */
        if ![
            "cmake-build-debug",
            "cmake-build-release",
            "cmake-build-relwithdebinfo",
            "cmake-build-minsizerel",
        ]
        .contains(&path.file_name_truncate())
        {
            return Ok(vec![]);
        }

        if !path.join("CMakeCache.txt").is_file() {
            return Ok(vec![]);
        }

        return Ok(vec![Box::new(DirectoryTarget::new(path.to_owned()))]);
    }
}
