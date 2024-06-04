use std::path::{
    Path,
    PathBuf,
};

use super::{
    CleanupResult,
    SizeEstimator,
    SweepableTarget,
};
use crate::{
    fs,
    SweeperError,
};

#[derive(Debug)]
pub struct DirectoryTarget {
    target_dir: PathBuf,
}

impl DirectoryTarget {
    pub fn new(target: PathBuf) -> Self {
        Self { target_dir: target }
    }
}

impl SweepableTarget for DirectoryTarget {
    fn name(&self) -> &str {
        "directory"
    }

    fn path(&self) -> &Path {
        &self.target_dir
    }

    fn estimated_size(&self) -> Box<SizeEstimator> {
        Box::new(fs::estimate_size_async(self.target_dir.clone()))
    }

    fn cleanup(&mut self, dry_run: bool) -> Result<CleanupResult, SweeperError> {
        let size_total = self.estimated_size().last();
        let result = CleanupResult {
            bytes_erased: size_total,
        };
        if dry_run {
            return Ok(result);
        }

        std::fs::remove_dir_all(&self.target_dir)?;
        Ok(result)
    }
}
