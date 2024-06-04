use std::{
    fmt::Debug,
    path::Path,
};

use crate::SweeperError;

pub type SizeEstimator = dyn Iterator<Item = u64> + Send + Sync;

#[derive(Debug)]
pub struct CleanupResult {
    pub bytes_erased: Option<u64>,
}

pub trait SweepableTarget: Send + Debug {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;

    fn estimated_size(&self) -> Box<SizeEstimator>;
    fn cleanup(&mut self, dry_run: bool) -> Result<CleanupResult, SweeperError>;
}

mod directory;
pub use directory::*;
