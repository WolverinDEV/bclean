use std::{
    path::PathBuf,
    sync::{
        mpsc::Receiver,
        Arc,
        Mutex,
    },
    thread::JoinHandle,
    time::{
        Duration,
        Instant,
    },
};

use bclean::{
    CrewOptions,
    CrewReport,
    CrewReportConsumer,
    SweepableTarget,
    SweeperCrew,
};
use crossterm::event::Event;
use ratatui::{
    buffer::Buffer,
    layout::{
        Constraint,
        Layout,
        Rect,
    },
    style::Stylize,
    text::{
        Line,
        Span,
    },
    widgets::Widget,
};

use super::TuiSweeperTargetSelect;
use crate::utils::format_duration;

#[derive(Debug, Default)]
struct UiReportInfo {
    current_file: Option<PathBuf>,
}
struct UiReportConsumer {
    ui_info: Arc<Mutex<UiReportInfo>>,
}

impl CrewReportConsumer for UiReportConsumer {
    fn consume_report(&mut self, report: CrewReport) {
        match report {
            CrewReport::StatusInspecting(target) => {
                let Ok(mut ui_info) = self.ui_info.lock() else {
                    return;
                };
                ui_info.current_file = Some(target);
            }
            _ => {}
        }
    }
}

pub struct SweeperWidget {
    time_started: Instant,
    time_total: Option<Duration>,

    root_path: PathBuf,

    crew_rx: Receiver<Box<dyn SweepableTarget>>,
    crew_handle: JoinHandle<()>,
    crew_finished: bool,

    target_select: TuiSweeperTargetSelect,
    report_info: Arc<Mutex<UiReportInfo>>,
}

impl SweeperWidget {
    pub fn new(root_path: PathBuf, crew: SweeperCrew, mut options: CrewOptions) -> Self {
        let report_info = Arc::new(Mutex::new(UiReportInfo::default()));
        options.report_consumer = Box::new(UiReportConsumer {
            ui_info: report_info.clone(),
        });

        let (crew_handle, crew_rx) = crew.execute(root_path.clone(), options);
        Self {
            root_path: root_path.clone(),

            time_started: Instant::now(),
            time_total: None,

            crew_rx,
            crew_handle,
            crew_finished: false,

            target_select: TuiSweeperTargetSelect::new(Some(root_path)),
            report_info,
        }
    }

    pub fn poll(&mut self) {
        while let Ok(value) = self.crew_rx.try_recv() {
            self.target_select.add_target(value);
        }

        let finished = self.crew_handle.is_finished();
        if finished == self.crew_finished {
            return;
        }

        /* we finished */
        self.time_total = Some(self.time_started.elapsed());
        self.crew_finished = true;
        log::debug!("Sweeper crew finished identifing targets");
    }

    pub fn handle_event(&mut self, event: &Event) {
        self.target_select.handle_event(event);
    }

    pub fn selected_target_count(&self) -> usize {
        self.target_select.selected_target_count()
    }

    pub fn remove_selected_targets(&mut self) -> Vec<Box<dyn SweepableTarget>> {
        self.target_select.remove_selected_targets()
    }
}

impl Widget for &SweeperWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let layout =
            Layout::vertical(&[Constraint::Percentage(100), Constraint::Length(1)]).split(area);

        let footer = {
            let time_elapsed = self
                .time_total
                .clone()
                .unwrap_or_else(|| self.time_started.elapsed());

            let mut line_segments = Vec::with_capacity(8);
            line_segments.push(Span::raw(format_duration(&time_elapsed)));

            if self.crew_finished {
                line_segments.push(Span::raw(format!(" Finished {}", self.root_path.display())));
            } else {
                let current_path = self
                    .report_info
                    .lock()
                    .map(|value| value.current_file.clone())
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| self.root_path.clone());

                line_segments.push(" Searching ".into());
                if let Ok(path) = current_path.strip_prefix(&self.root_path) {
                    line_segments.push(Span::raw(format!("{}", self.root_path.join("").display())));
                    line_segments.push(Span::raw(format!("{}", path.display())).italic());
                } else {
                    line_segments.push(Span::raw(format!("{}", current_path.display())));
                }
            };

            let text = Line::from(line_segments);
            if self.crew_finished {
                text.green()
            } else {
                text.blue()
            }
        };

        self.target_select.render(layout[0], buf);
        footer.render(layout[1], buf);
    }
}
