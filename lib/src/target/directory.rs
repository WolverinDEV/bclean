use std::{
    path::{
        Path,
        PathBuf,
    },
    thread,
    time::Duration,
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

    fn cleanup(self, dry_run: bool) -> Result<CleanupResult, SweeperError> {
        if dry_run {
            thread::sleep(Duration::from_secs(10));
            return Ok(CleanupResult { bytes_erased: 0 });
        }
        todo!()
    }
}
