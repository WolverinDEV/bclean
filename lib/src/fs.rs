use std::{
    fs::{self, DirEntry},
    io, iter,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

pub(crate) struct DirWalker {
    pending_entries: Vec<DirEntry>,
}

impl DirWalker {
    pub fn new() -> Self {
        Self {
            pending_entries: Vec::with_capacity(1024),
        }
    }

    pub fn next_item(&mut self) -> Option<DirEntry> {
        self.pending_entries.pop()
    }

    pub fn insert_entries(&mut self, path: &Path) -> io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_err) => continue,
            };

            self.pending_entries.push(entry);
        }

        Ok(())
    }
}

pub fn estimate_size_async(dir: PathBuf) -> impl Iterator<Item = u64> {
    let mut walker = DirWalker::new();
    let _ = walker.insert_entries(&dir);

    let mut size_iter = iter::from_fn(move || {
        while let Some(current_entry) = walker.next_item() {
            let file_meta = match current_entry.metadata() {
                Ok(meta) => meta,
                Err(_) => continue,
            };

            if file_meta.is_file() {
                return Some(file_meta.file_size());
            }

            let _ = walker.insert_entries(&current_entry.path());
        }
        None
    });

    let mut size_total = 0;
    iter::from_fn(move || {
        size_total += size_iter.next()?;
        for _ in 0..1_000 {
            match size_iter.next() {
                Some(file_size) => size_total += file_size,
                None => break,
            }
        }

        Some(size_total)
    })
}
