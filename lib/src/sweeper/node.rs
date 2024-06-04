use std::path::{Path, PathBuf};

use crate::{fs, path::PathEx};

use super::{SizeEstimator, SweepableTarget, Sweeper, SweeperError};

pub struct NodeSweeper;

impl NodeSweeper {
    pub fn new() -> Self {
        Self
    }
}

impl Sweeper for NodeSweeper {
    fn name(&self) -> &str {
        "node_modules"
    }

    fn identify_targets(&self, path: &Path) -> Result<Vec<Box<dyn SweepableTarget>>, SweeperError> {
        if !path.is_dir() || path.file_name_truncate() != "node_modules" {
            return Ok(vec![]);
        }

        // package.json
        let parent = match path.parent() {
            Some(parent) => parent,
            None => return Ok(vec![]),
        };
        if !parent
            .contains_file_ignore_case("package.json")
            .unwrap_or(false)
        {
            return Ok(vec![]);
        }

        Ok(vec![Box::new(NodeTarget {
            target_dir: path.to_owned(),
        })])
    }
}

#[derive(Debug)]
pub struct NodeTarget {
    target_dir: PathBuf,
}

impl SweepableTarget for NodeTarget {
    fn name(&self) -> &str {
        "node_modules"
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
