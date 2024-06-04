use std::{ffi::OsStr, fs::DirEntry, io, path::Path};

static EMPTY_STR: &'static str = "";

/// Utility functions for the systems path library
pub trait PathEx {
    /// Returns the file name from the current path, or an empty string if the file name is empty
    fn file_name_truncate(&self) -> &str;

    /// Tests if the path is a directory and contains a certail file but ignoring the name casing
    fn contains_file_ignore_case(&self, file_name: &str) -> io::Result<bool>;
}

impl PathEx for &Path {
    fn file_name_truncate(&self) -> &str {
        self.file_name()
            .map(OsStr::to_str)
            .flatten()
            .unwrap_or(EMPTY_STR)
    }

    fn contains_file_ignore_case(&self, file_name: &str) -> io::Result<bool> {
        let file_name = file_name.to_lowercase();
        let contains_file = self
            .read_dir()?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.is_file())
            .find(|entry| entry.file_name().to_string_lossy().to_lowercase() == file_name)
            .is_some();

        Ok(contains_file)
    }
}

pub trait DirEntryEx {
    fn is_file(&self) -> bool;
    fn is_dir(&self) -> bool;
}

impl DirEntryEx for DirEntry {
    fn is_file(&self) -> bool {
        self.file_type()
            .map_or(false, |file_type| file_type.is_file())
    }

    fn is_dir(&self) -> bool {
        self.file_type()
            .map_or(false, |file_type| file_type.is_dir())
    }
}
