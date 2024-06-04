use std::path::Path;

use super::{
    SweepableTarget,
    Sweeper,
};
use crate::{
    path::PathEx,
    sweeper::SweeperError,
    target::DirectoryTarget,
};

pub struct RustSweeper;

impl RustSweeper {
    pub fn new() -> Self {
        Self
    }
}

impl Sweeper for RustSweeper {
    fn name(&self) -> &str {
        "rust"
    }

    fn identify_targets(&self, dir: &Path) -> Result<Vec<Box<dyn SweepableTarget>>, SweeperError> {
        if dir.file_name_truncate() != "target" {
            return Ok(vec![]);
        }

        if !dir.join("CACHEDIR.TAG").is_file() {
            return Ok(vec![]);
        }

        if !dir.join(".rustc_info.json").is_file() {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(DirectoryTarget::new(dir.to_owned()))])
    }
}
