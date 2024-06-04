use std::{
    borrow::Cow,
    cell::RefCell,
    collections::BTreeMap,
    io::stdout,
    path::PathBuf,
    str::FromStr,
    sync::{
        atomic::{AtomicI64, Ordering},
        mpsc::{self, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use bclean::{
    sweeper::{NodeSweeper, RustSweeper, SizeEstimator},
    utils, CrewOptions, SweepableTarget, SweeperCrew,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, Span, Text},
    widgets::{Block, Cell, Paragraph, Row, Table, Widget},
    Terminal,
};
use tui_logger::{Drain, TuiLoggerWidget};

struct ScrollableText {
    baseline: Instant,

    /// The text scroll speed in characters per second
    scroll_speed: f32,

    overscroll_start: usize,
    overscroll_end: usize,

    text: String,
}

impl ScrollableText {
    pub fn new(text: String) -> Self {
        Self {
            text,
            baseline: Instant::now(),

            scroll_speed: 6.0,
            overscroll_start: 5,
            overscroll_end: 5,
        }
    }

    pub fn reset_scroll(&mut self) {
        self.baseline = Instant::now();
    }

    pub fn display_value(&self, max_width: usize) -> Cow<str> {
        if self.text.len() <= max_width {
            return (&self.text).into();
        }

        let sequence_length =
            self.overscroll_start + self.text.len() - max_width + self.overscroll_end;

        let time_offset = self.baseline.elapsed().as_millis() as f32 / 1000.0;
        let mut char_offset = (time_offset * self.scroll_speed) as usize % sequence_length;
        if char_offset < self.overscroll_start {
            char_offset = 0;
        } else if char_offset > self.overscroll_start + self.text.len() - max_width {
            char_offset = self.text.len() - max_width;
        } else {
            char_offset -= self.overscroll_start;
        }

        self.text[char_offset..char_offset + max_width].into()
    }

    pub fn fixed_value(&self, max_width: usize) -> Cow<str> {
        if self.text.len() <= max_width {
            return (&self.text).into();
        } else if max_width >= 3 {
            return format!("{}...", &self.text[0..max_width - 3]).into();
        } else {
            return "..."[0..max_width].into();
        }
    }
}

fn _dummy() {
    let options = CrewOptions::default();

    let mut crew = SweeperCrew::new();
    crew.register(RustSweeper::new());
    crew.register(NodeSweeper::new());

    let (_handle, rx) = crew.execute(
        PathBuf::from_str(r#"C:\Users\Markus\git"#).unwrap(),
        options,
    );

    let mut size_total = 0;
    while let Ok(target) = rx.recv() {
        let entry_size = target.estimated_size().last().unwrap_or(0);
        log::info!(
            "- {}: {} | {}",
            target.name(),
            target.path().display(),
            utils::format_file_size(entry_size)
        );

        size_total += entry_size;
    }

    log::info!(
        "{} total removeable files",
        utils::format_file_size(size_total)
    );
}

struct TuiTargetSelectState {
    _target_id: u32,
    target: Box<dyn SweepableTarget>,
    size: Arc<AtomicI64>,
    selected: bool,

    ui_path: ScrollableText,
}

struct TuiSweeperTargetSelect {
    target_id_index: u32,
    targets: BTreeMap<u32, TuiTargetSelectState>,

    cursor_current: usize,
    view_offset: usize,
    view_height: RefCell<usize>,

    estimate_handle: Option<JoinHandle<()>>,
    estimate_tx: Option<Sender<(Box<SizeEstimator>, Arc<AtomicI64>)>>,
}

impl TuiSweeperTargetSelect {
    pub fn new() -> Self {
        let (estimate_tx, estimate_rx) = mpsc::channel::<(Box<SizeEstimator>, Arc<AtomicI64>)>();
        let estimate_handle = thread::spawn(move || {
            while let Ok((estimator, target_value)) = estimate_rx.recv() {
                let value = estimator
                    .inspect(|value| {
                        target_value.store(-(*value as i64), Ordering::Relaxed);
                    })
                    .last()
                    .unwrap_or(0);

                target_value.store(value as i64, Ordering::Relaxed);
            }
        });

        Self {
            target_id_index: 0,
            targets: Default::default(),

            cursor_current: 0,
            view_offset: 0,
            view_height: RefCell::new(100),

            estimate_handle: Some(estimate_handle),
            estimate_tx: Some(estimate_tx),
        }
    }

    pub fn add_target(&mut self, target: Box<dyn SweepableTarget>) {
        self.target_id_index += 1;
        let target_id = self.target_id_index;

        let target = TuiTargetSelectState {
            ui_path: ScrollableText::new(format!("{}", target.path().display())),

            _target_id: target_id,
            target,

            selected: false,
            size: Default::default(),
        };

        if let Some(tx) = &self.estimate_tx {
            let _ = tx.send((target.target.estimated_size(), target.size.clone()));
        }

        self.targets.insert(target_id, target);
    }

    fn cursor_target_mut(&mut self) -> Option<&mut TuiTargetSelectState> {
        self.cursor_target_id()
            .map(|target_id| self.targets.get_mut(&target_id))
            .flatten()
    }

    fn cursor_target_id(&self) -> Option<u32> {
        self.targets
            .keys()
            .skip(self.cursor_current)
            .next()
            .cloned()
    }

    pub fn handle_event(&mut self, event: &Event) {
        let Event::Key(event) = event else { return };
        if event.code == KeyCode::Char(' ') && event.kind == KeyEventKind::Press {
            if let Some(target) = self.cursor_target_mut() {
                target.selected = !target.selected;
            }
        }

        if event.code == KeyCode::Down
            && matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            && self.cursor_current + 1 < self.targets.len()
        {
            self.set_cursor_index(self.cursor_current + 1);
        }

        if event.code == KeyCode::Up
            && matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            && self.cursor_current > 0
        {
            self.set_cursor_index(self.cursor_current - 1);
        }
    }

    fn set_cursor_index(&mut self, index: usize) {
        let index = index.clamp(0, self.targets.len() - 1);
        let view_height = *self.view_height.borrow();

        if index >= self.view_offset + view_height {
            self.view_offset = index - view_height + 1;
        }
        if index < self.view_offset + 1 {
            self.view_offset = if index > 0 { index - 1 } else { 0 };
        }

        self.cursor_current = index;
        if let Some(target) = self.cursor_target_mut() {
            target.ui_path.reset_scroll();
        }
    }
}

impl Widget for &TuiSweeperTargetSelect {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .split(area);
        *self.view_height.borrow_mut() = layout[1].height as usize;

        let header = { Line::raw(format!("Total targets: {}", self.targets.len())) };

        let content = {
            let mut rows = Vec::with_capacity(layout[0].height as usize);

            let width_layout = Layout::horizontal([
                Constraint::Length(16), // Checkbox + space + size + space
                Constraint::Fill(1),    // Path name
                Constraint::Length(18), // Target name + left space
            ])
            .split(layout[1]);

            let max_path_text_width = width_layout[1].width as usize;

            for (index, target) in self.targets.values().enumerate().skip(self.view_offset) {
                let target_size = target.size.load(Ordering::Relaxed);
                let target_selected = if target.selected { "X" } else { " " };

                let target_size = if target_size > 0 {
                    utils::format_file_size(target_size as u64)
                } else if target_size < 0 {
                    /* estimate */
                    utils::format_file_size(target_size.abs() as u64)
                } else {
                    "waiting".to_string()
                };

                let row = Row::new(vec![
                    Cell::new(format!("[{}]", target_selected)),
                    Cell::new(Text::raw(target_size).alignment(Alignment::Right)),
                    Cell::new(if index == self.cursor_current {
                        target.ui_path.display_value(max_path_text_width)
                    } else {
                        target.ui_path.fixed_value(max_path_text_width)
                    }),
                    Cell::new(target.target.name()),
                ]);
                let row = if index == self.cursor_current {
                    row.on_gray()
                } else {
                    row
                };

                rows.push(row);
            }

            Table::new(
                rows,
                &[
                    Constraint::Length(3),
                    Constraint::Length(12),
                    Constraint::Fill(1),
                    Constraint::Length(16),
                ],
            )
            .header(Row::new(vec![
                Cell::new(""),
                Cell::new("Size"),
                Cell::new("Path"),
                Cell::new("Type"),
            ]))
        };

        let footer = {
            let size_total = self
                .targets
                .values()
                .map(|target| target.size.load(Ordering::Relaxed).abs() as u64)
                .sum::<u64>();

            let count_selected = self
                .targets
                .values()
                .filter(|target| target.selected)
                .count();
            let size_selected = self
                .targets
                .values()
                .filter(|target| target.selected)
                .map(|target| target.size.load(Ordering::Relaxed).abs() as u64)
                .sum::<u64>();

            let text_total = Span::raw(format!(
                "{} total {}",
                self.targets.len(),
                utils::format_file_size(size_total),
            ));

            let text_selected = if count_selected == 0 {
                Span::raw(format!("No selection"))
            } else {
                Span::raw(format!(
                    "{} selected {}",
                    count_selected,
                    utils::format_file_size(size_selected),
                ))
            };
            Paragraph::new(Line::from_iter([text_total, " | ".into(), text_selected]))
        };

        header.render(layout[0], buf);
        content.render(layout[1], buf);
        footer.render(layout[2], buf);
    }
}

impl Drop for TuiSweeperTargetSelect {
    fn drop(&mut self) {
        self.estimate_tx = None;
        if let Some(handle) = self.estimate_handle.take() {
            let _ = handle.join();
        }
    }
}

fn main() -> anyhow::Result<()> {
    let tui_log_drain = Drain::new();
    env_logger::builder()
        .format(move |_buf, record| Ok(tui_log_drain.log(record)))
        .init();

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let crew_rx = {
        let options = CrewOptions::default();

        let mut crew = SweeperCrew::new();
        crew.register(RustSweeper::new());
        crew.register(NodeSweeper::new());

        let (_handle, rx) = crew.execute(
            PathBuf::from_str(r#"C:\Users\Markus\git"#).unwrap(),
            options,
        );

        rx
    };

    let mut target_select = TuiSweeperTargetSelect::new();

    loop {
        terminal.draw(|frame| {
            let layout =
                Layout::horizontal(&[Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(frame.size());
            frame.render_widget(&target_select, layout[0]);

            frame.render_widget(Block::new().on_blue(), layout[1]);
            frame.render_widget(TuiLoggerWidget::default(), layout[1]);
        })?;

        if let Ok(value) = crew_rx.try_recv() {
            target_select.add_target(value);
        }

        if event::poll(std::time::Duration::from_millis(16))? {
            let event = event::read()?;
            target_select.handle_event(&event);
            if let event::Event::Key(key) = event {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
