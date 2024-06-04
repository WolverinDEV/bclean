use std::path::Path;

use super::{
    SweepableTarget,
    Sweeper,
    SweeperError,
};
use crate::{
    path::PathEx,
    target::DirectoryTarget,
};

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

        Ok(vec![Box::new(DirectoryTarget::new(path.to_owned()))])
    }
}
