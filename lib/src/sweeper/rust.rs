use std::path::{Path, PathBuf};

use crate::{fs, path::PathEx, sweeper::SweeperError};

use super::{SizeEstimator, SweepableTarget, Sweeper};

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

        Ok(vec![Box::new(RustTarget {
            target_dir: dir.to_owned(),
        })])
    }
}

#[derive(Debug)]
pub struct RustTarget {
    target_dir: PathBuf,
}

impl SweepableTarget for RustTarget {
    fn name(&self) -> &str {
        "rust target"
    }

    fn path(&self) -> &Path {
        &self.target_dir
    }

    fn estimated_size(&self) -> Box<SizeEstimator> {
        Box::new(fs::estimate_size_async(self.target_dir.clone()))
    }

    fn cleanup(self, _dry_run: bool) -> Result<super::CleanupResult, SweeperError> {
        todo!()
    }
}
