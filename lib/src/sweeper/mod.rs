use std::{error, fmt::Debug, io, path::Path};

use thiserror::Error;

pub type SizeEstimator = dyn Iterator<Item = u64> + Send + Sync;

pub trait SweepableTarget: Send + Debug {
    fn name(&self) -> &str;
    fn path(&self) -> &Path;

    fn estimated_size(&self) -> Box<SizeEstimator>;
    fn cleanup(self, dry_run: bool) -> Result<CleanupResult, SweeperError>;
}

/// A Sweeper implements a cleanup mechanism for a specific language or build artefact.
pub trait Sweeper: Sync + Send {
    /// Name of the sweeper
    fn name(&self) -> &str;

    /// Find all possible targets for the sweeper based on the target options.
    fn identify_targets(
        &self,
        directory: &Path,
    ) -> Result<Vec<Box<dyn SweepableTarget>>, SweeperError>;
}

#[derive(Error, Debug)]
pub enum SweeperError {
    #[error("io: {0}")]
    IoError(#[from] io::Error),

    #[error("{0}")]
    Other(#[from] Box<dyn error::Error + Send>),
}

pub struct CleanupResult {
    pub bytes_erased: usize,
}

mod rust;
pub use rust::*;

mod node;
pub use node::*;
