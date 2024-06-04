use std::path::PathBuf;

use bclean::{
    CrewOptions,
    SweeperCrew,
};
use crossterm::event::{
    Event,
    KeyCode,
    KeyEventKind,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::Text,
    widgets::{
        Block,
        Borders,
        Clear,
        Paragraph,
        Widget,
        Wrap,
    },
};

use super::{
    SweeperWidget,
    SweepingWidget,
};
use crate::utils;

pub enum AppView {
    TargetSelect {
        view: SweeperWidget,
        show_no_selection: bool,
        dry_run: bool,
    },
    Sweeping {
        view: SweepingWidget,
    },
}

impl AppView {
    pub fn new(root_path: PathBuf, crew: SweeperCrew, options: CrewOptions, dry_run: bool) -> Self {
        Self::TargetSelect {
            view: SweeperWidget::new(root_path, crew, options),
            show_no_selection: false,
            dry_run: dry_run,
        }
    }

    pub fn poll(&mut self) {
        match self {
            Self::TargetSelect { view, .. } => view.poll(),
            Self::Sweeping { view, .. } => view.poll(),
        }
    }

    pub fn handle_event(&mut self, event: &Event) {
        match self {
            Self::TargetSelect {
                view,
                show_no_selection,
                dry_run,
            } => {
                if let Event::Key(key) = event {
                    if key.kind == KeyEventKind::Press && key.code == KeyCode::Enter {
                        if *show_no_selection {
                            *show_no_selection = false;
                            return;
                        }

                        if view.selected_target_count() == 0 {
                            *show_no_selection = true;
                            return;
                        }

                        *self = Self::Sweeping {
                            view: SweepingWidget::new(view.remove_selected_targets(), *dry_run),
                        };
                        return;
                    }
                }

                view.handle_event(event);
            }
            Self::Sweeping { view, .. } => view.handle_event(event),
        }
    }
}

impl Widget for &AppView {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        match self {
            AppView::TargetSelect {
                view,
                show_no_selection,
                ..
            } => {
                view.render(area, buf);
                if *show_no_selection {
                    let block = Block::default()
                        .title("No targets")
                        .borders(Borders::ALL)
                        .on_gray();
                    let area = utils::centered_rect(60, 20, area);

                    let popup =
                        Paragraph::new(Text::raw("Please select at least one target to sweep"))
                            .wrap(Wrap { trim: true })
                            .block(block);

                    Clear::render(Clear, area, buf);
                    popup.render(area, buf);
                }
            }
            AppView::Sweeping { view } => view.render(area, buf),
        }
    }
}
