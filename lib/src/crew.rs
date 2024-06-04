use std::{
    io,
    path::PathBuf,
    sync::mpsc::{
        self,
        Receiver,
    },
    thread::{
        self,
        JoinHandle,
    },
};

use crate::{
    target::SweepableTarget,
    DirEntryEx,
    DirWalker,
    Sweeper,
    SweeperError,
};

pub struct CrewOptions {
    /// Keep searching the paths subdirectories even tough the parent directory
    /// has already been identified as a sweepable target.
    pub search_recursively: bool,

    pub report_consumer: Box<dyn CrewReportConsumer + Send>,
}

impl Default for CrewOptions {
    fn default() -> Self {
        Self {
            search_recursively: false,
            report_consumer: Box::new(VoidCrewReportConsumer),
        }
    }
}

pub trait CrewReportConsumer {
    fn consume_report(&mut self, report: CrewReport);
}

pub struct VoidCrewReportConsumer;
impl CrewReportConsumer for VoidCrewReportConsumer {
    fn consume_report(&mut self, _report: CrewReport) {}
}

pub enum CrewReport {
    StatusInspecting(PathBuf),
    ErrorSweeper {
        sweeper: String,
        error: SweeperError,
    },
    ErrorFs(io::Error),
}

pub struct SweeperCrew {
    members: Vec<Box<dyn Sweeper>>,
}

impl SweeperCrew {
    pub fn new() -> Self {
        Self {
            members: Vec::new(),
        }
    }

    pub fn register<T: Sweeper + 'static>(&mut self, sweeper: T) {
        self.register_boxed(Box::new(sweeper))
    }

    pub fn register_boxed(&mut self, sweeper: Box<dyn Sweeper>) {
        self.members.push(sweeper)
    }

    pub fn execute(
        self,
        root_directory: PathBuf,
        mut options: CrewOptions,
    ) -> (JoinHandle<()>, Receiver<Box<dyn SweepableTarget>>) {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut dir_walker = DirWalker::new();
            if let Err(error) = dir_walker.insert_entries(&root_directory) {
                log::warn!("Failed to read root dir: {:#}", error);
                options
                    .report_consumer
                    .consume_report(CrewReport::ErrorFs(error));
            }

            'search_loop: while let Some(item) = dir_walker.next_item() {
                let item_path = item.path();
                let mut target_found = false;

                options
                    .report_consumer
                    .consume_report(CrewReport::StatusInspecting(item_path.clone()));

                for sweeper in &self.members {
                    let targets = match sweeper.identify_targets(&item_path) {
                        Ok(targets) => targets,
                        Err(error) => {
                            log::warn!(
                                "Sweeper {} failed for {}: {:#}",
                                sweeper.name(),
                                item_path.display(),
                                error
                            );
                            options
                                .report_consumer
                                .consume_report(CrewReport::ErrorSweeper {
                                    error,
                                    sweeper: sweeper.name().to_string(),
                                });
                            continue;
                        }
                    };

                    for target in targets {
                        log::trace!(
                            "Identified new target {} at {} by {}",
                            target.name(),
                            target.path().display(),
                            sweeper.name()
                        );
                        target_found = true;

                        if tx.send(target).is_err() {
                            /* Abort search */
                            log::debug!("Aborting search as receiving end has been closed");
                            break 'search_loop;
                        }
                    }
                }

                if item.is_dir() && (options.search_recursively || !target_found) {
                    if let Err(error) = dir_walker.insert_entries(&item_path) {
                        log::warn!(
                            "Failed to read directory {}: {:#}",
                            item_path.display(),
                            error
                        );
                        options
                            .report_consumer
                            .consume_report(CrewReport::ErrorFs(error));
                    }
                }
            }
        });

        (handle, rx)
    }
}
