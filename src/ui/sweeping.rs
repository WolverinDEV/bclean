use std::{
    mem,
    path::PathBuf,
    sync::{
        Arc,
        Mutex,
    },
    thread::{
        self,
        JoinHandle,
    },
    time::{
        Duration,
        Instant,
    },
};

use bclean::{
    utils,
    CleanupResult,
    SweepableTarget,
    SweeperError,
};
use crossterm::event::{
    Event,
    KeyCode,
    KeyEventKind,
};
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
    widgets::{
        Paragraph,
        Widget,
    },
};

use crate::utils::format_duration;

struct SpinerText {
    baseline: Instant,
    speed: f32,
}

impl SpinerText {
    pub fn new() -> Self {
        Self {
            baseline: Instant::now(),
            speed: 5.0,
        }
    }

    pub fn current_text(&self) -> &str {
        let offset = (self.baseline.elapsed().as_millis() as f32 / 1000.0 * self.speed).floor()
            as usize
            % SPINER_CHAR_SEQUENCE.len();
        SPINER_CHAR_SEQUENCE[offset]
    }
}

const SPINER_CHAR_SEQUENCE: [&'static str; 8] = ["⣷", "⣯", "⣟", "⡿", "⢿", "⣻", "⣽", "⣾"];

enum SweepingTargetState {
    Pending(Box<dyn SweepableTarget>),
    Executing,
    Cleaned {
        result: CleanupResult,
        time: Duration,
    },
    Failed(SweeperError),
}

struct SweepingTarget {
    state: Arc<Mutex<SweepingTargetState>>,
    path: PathBuf,
}

pub struct SweepingWidget {
    targets: Vec<SweepingTarget>,
    dry_run: bool,

    executor: Option<JoinHandle<()>>,

    text_spinner: SpinerText,
}

impl SweepingWidget {
    pub fn new(targets: Vec<Box<dyn SweepableTarget>>, dry_run: bool) -> Self {
        let targets = targets
            .into_iter()
            .map(|target| SweepingTarget {
                path: target.path().to_owned(),
                state: Arc::new(Mutex::new(SweepingTargetState::Pending(target))),
            })
            .collect::<Vec<_>>();

        Self {
            targets,
            executor: None,
            dry_run,

            text_spinner: SpinerText::new(),
        }
    }

    pub fn poll(&mut self) {}

    pub fn handle_event(&mut self, event: &Event) {
        let Event::Key(key) = event else { return };

        if key.code == KeyCode::Enter && key.kind == KeyEventKind::Press {
            self.execute();
        }
    }

    fn execute(&mut self) {
        if self.executor.is_some() {
            return;
        }

        let executor_targets = self
            .targets
            .iter()
            .map(|target| target.state.clone())
            .collect::<Vec<_>>();

        let dry_run = self.dry_run;
        let executor = thread::spawn(move || {
            for target_state in executor_targets {
                let mut target = {
                    let Ok(mut state) = target_state.lock() else {
                        continue;
                    };

                    match mem::replace(&mut *state, SweepingTargetState::Executing) {
                        SweepingTargetState::Pending(target) => target,
                        orig_state => {
                            *state = orig_state;
                            continue;
                        }
                    }
                };

                let time_start = Instant::now();
                let target_path = target.path().to_owned();
                let result = target.cleanup(dry_run);
                log::debug!("Target {} -> {:#?}", target_path.display(), result);

                let Ok(mut state) = target_state.lock() else {
                    continue;
                };
                *state = match result {
                    Ok(result) => SweepingTargetState::Cleaned {
                        result,
                        time: time_start.elapsed(),
                    },
                    Err(err) => SweepingTargetState::Failed(err),
                };
            }

            log::debug!("Finished cleaning up targets.")
        });

        self.executor = Some(executor);
    }
}

impl Widget for &SweepingWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let layout = Layout::vertical(&[Constraint::Length(1), Constraint::Fill(1)]).split(area);

        let mut targets_finished = 0;
        let mut lines = Vec::new();
        for target in self.targets.iter() {
            let Ok(status) = target.state.lock() else {
                continue;
            };
            let status_icon = match &*status {
                SweepingTargetState::Pending(_) => Span::raw(" "),
                SweepingTargetState::Executing => {
                    Span::raw(self.text_spinner.current_text()).blue()
                }
                SweepingTargetState::Failed(_) => Span::raw("X").red(),
                SweepingTargetState::Cleaned { .. } => Span::raw("✓").green(),
            };

            let status_text = match &*status {
                SweepingTargetState::Pending(_) => Span::raw("pending").italic(),
                SweepingTargetState::Executing => Span::raw("executing").blue(),
                SweepingTargetState::Failed(error) => Span::raw(format!("{:#}", error)).red(),
                SweepingTargetState::Cleaned { result, time } => Span::raw(format!(
                    "finished: {} cleaned in {}",
                    utils::format_file_size(result.bytes_erased.unwrap_or(0)),
                    format_duration(time)
                ))
                .green(),
            };

            if !matches!(&*status, SweepingTargetState::Pending(_)) {
                targets_finished += 1;
            }

            lines.push(Line::from(vec![
                "[".into(),
                status_icon,
                format!("] {}", target.path.display()).into(),
            ]));
            lines.push(Line::from(vec!["    ↳ ".into(), status_text]));
        }

        let title = {
            let text = if self.executor.is_some() {
                if self.dry_run {
                    format!(
                        "Sweeping {}/{} targets (dry run)",
                        targets_finished,
                        self.targets.len()
                    )
                } else {
                    format!(
                        "Sweeping {}/{} targets",
                        targets_finished,
                        self.targets.len()
                    )
                }
            } else {
                format!(
                    "Sweeping {} targets. Press 'Enter' again to start{}",
                    self.targets.len(),
                    if self.dry_run { " (dry run)" } else { "" }
                )
            };
            Line::raw(text)
        };

        title.render(layout[0], buf);
        Paragraph::new(lines).render(layout[1], buf);
    }
}
