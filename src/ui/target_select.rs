use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{
        Path,
        PathBuf,
    },
    sync::{
        atomic::{
            AtomicI64,
            Ordering,
        },
        mpsc::{
            self,
            Sender,
        },
        Arc,
    },
    thread::{
        self,
        JoinHandle,
    },
};

use bclean::{
    SizeEstimator,
    SweepableTarget,
};
use crossterm::event::{
    Event,
    KeyCode,
    KeyEventKind,
};
use ratatui::{
    buffer::Buffer,
    layout::{
        Alignment,
        Constraint,
        Layout,
        Rect,
    },
    style::Stylize,
    text::{
        Line,
        Span,
        Text,
    },
    widgets::{
        Cell,
        Paragraph,
        Row,
        Table,
        Widget,
    },
};

use super::ScrollableText;
use crate::utils;

struct TuiTargetSelectState {
    _target_id: u32,
    target: Box<dyn SweepableTarget>,
    size: Arc<AtomicI64>,
    selected: bool,

    ui_path: ScrollableText,
}

pub struct TuiSweeperTargetSelect {
    target_id_index: u32,
    targets: BTreeMap<u32, TuiTargetSelectState>,

    cursor_current: usize,
    view_offset: usize,
    view_height: RefCell<usize>,

    _estimate_handle: Option<JoinHandle<()>>,
    estimate_tx: Option<Sender<(Box<SizeEstimator>, Arc<AtomicI64>)>>,

    strip_root_path: Option<PathBuf>,

    select_all: bool,
}

impl TuiSweeperTargetSelect {
    pub fn new(strip_root_path: Option<PathBuf>) -> Self {
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

            _estimate_handle: Some(estimate_handle),
            estimate_tx: Some(estimate_tx),

            strip_root_path,

            select_all: false,
        }
    }

    pub fn add_target(&mut self, target: Box<dyn SweepableTarget>) {
        self.target_id_index += 1;
        let target_id = self.target_id_index;

        let path_text = if let Some(root_path) = &self.strip_root_path {
            if let Ok(path) = target.path().strip_prefix(root_path) {
                format!("{}", Path::new(".").join(path).display())
            } else {
                format!("{}", target.path().display())
            }
        } else {
            format!("{}", target.path().display())
        };

        let target = TuiTargetSelectState {
            ui_path: ScrollableText::new(path_text),

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

    pub fn selected_target_count(&self) -> usize {
        self.targets
            .values()
            .filter(|target| target.selected)
            .count()
    }

    pub fn remove_selected_targets(&mut self) -> Vec<Box<dyn SweepableTarget>> {
        // As soon as #70530 get's stabalized, we can use this instead:
        // self.targets
        //     .extract_if(|_target_id, target| target.selected)
        //     .map(|(_target_id, target)| target.target)
        //     .collect()

        let selected_targets = self
            .targets
            .iter()
            .filter(|(_, target)| target.selected)
            .map(|(target_id, _)| *target_id)
            .collect::<Vec<_>>();

        let mut removed_targets = Vec::new();
        for target_id in selected_targets {
            let Some(target) = self.targets.remove(&target_id) else {
                continue;
            };
            removed_targets.push(target.target);
        }
        removed_targets
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
            if self.select_all {
                let selected = self
                    .cursor_target_mut()
                    .map_or(false, |target| target.selected);
                for target in self.targets.values_mut() {
                    target.selected = !selected;
                }
            } else {
                if let Some(target) = self.cursor_target_mut() {
                    target.selected = !target.selected;
                }
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

        if event.code == KeyCode::PageDown && event.kind == KeyEventKind::Press {
            self.set_cursor_index(self.targets.len());
        }

        if event.code == KeyCode::PageUp && event.kind == KeyEventKind::Press {
            self.set_cursor_index(0);
        }

        if event.code == KeyCode::Char('a') && event.kind == KeyEventKind::Press {
            self.select_all = !self.select_all;
        }
    }

    fn set_cursor_index(&mut self, index: usize) {
        let index = index.clamp(0, self.targets.len() - 1);
        let view_height = *self.view_height.borrow() - 1;

        if index >= self.view_offset + view_height {
            self.view_offset = index - view_height + 1;
        }
        if index < self.view_offset + 1 {
            self.view_offset = if index > 0 { index - 1 } else { 0 };
        }

        self.cursor_current = index;
        self.select_all = false;
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
        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).split(area);
        *self.view_height.borrow_mut() = layout[0].height as usize;

        let content = {
            let mut rows = Vec::with_capacity(layout[0].height as usize);

            let width_layout = Layout::horizontal([
                Constraint::Length(16), // Checkbox + space + size + space
                Constraint::Fill(1),    // Path name
                Constraint::Length(18), // Target name + left space
            ])
            .split(layout[0]);

            let max_path_text_width = width_layout[1].width as usize;

            for (index, target) in self.targets.values().enumerate().skip(self.view_offset) {
                let target_size = target.size.load(Ordering::Relaxed);
                let target_selected = if target.selected { "X" } else { " " };

                let target_size = if target_size > 0 {
                    utils::format_file_size(target_size as u64).into()
                } else if target_size < 0 {
                    /* estimate */
                    Span::raw(utils::format_file_size(target_size.abs() as u64)).italic()
                } else {
                    "waiting".into()
                };

                let row = Row::new(vec![
                    Cell::new(format!("[{}]", target_selected)),
                    Cell::new(Text::from(target_size).alignment(Alignment::Right)),
                    Cell::new(if index == self.cursor_current {
                        target.ui_path.display_value(max_path_text_width)
                    } else {
                        target.ui_path.fixed_value(max_path_text_width)
                    }),
                    Cell::new(target.target.name()),
                ]);
                let row = if index == self.cursor_current || self.select_all {
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

        content.render(layout[0], buf);
        footer.render(layout[1], buf);
    }
}
