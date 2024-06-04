#![feature(btree_extract_if)]

use std::{
    self,
    env,
    io::stdout,
    path::PathBuf,
};

use bclean::{
    sweeper::{
        CMakeSweeper,
        NodeSweeper,
        RustSweeper,
    },
    CrewOptions,
    SweeperCrew,
};
use clap::{
    command,
    Parser,
};
use crossterm::{
    event::{
        self,
        Event,
        KeyCode,
        KeyEventKind,
    },
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{
    self,
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{
        Constraint,
        Direction,
        Layout,
        Rect,
    },
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
    Terminal,
};
use tui_logger::{
    Drain,
    TuiLoggerWidget,
};
use ui::{
    SweeperWidget,
    SweepingWidget,
};

mod ui;
mod utils;

enum AppView {
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

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
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
                    let area = centered_rect(60, 20, area);

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

struct TuiAppLoggerWidget;

impl Widget for TuiAppLoggerWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new().title("Logging output").borders(Borders::LEFT);
        TuiLoggerWidget::default().block(block).render(area, buf);
    }
}

/// Automate the cleanup of left over build files
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    root: Option<PathBuf>,

    #[arg(long)]
    ui_logger: bool,

    #[arg(short, long)]
    dry_run: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.ui_logger {
        let tui_log_drain = Drain::new();
        env_logger::builder()
            .format(move |_buf, record| Ok(tui_log_drain.log(record)))
            .init();
    } else {
        env_logger::init();
    }

    let root_path = args.root.unwrap_or_else(|| env::current_dir().unwrap());
    let root_path = match dunce::canonicalize(root_path) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Invalid root path: {:#}", err);
            return Ok(());
        }
    };
    log::debug!("Root path: {}", root_path.display());

    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;

    let crew = {
        let mut crew = SweeperCrew::new();
        crew.register(RustSweeper::new());
        crew.register(NodeSweeper::new());
        crew.register(CMakeSweeper::new());

        crew
    };

    let mut view = AppView::TargetSelect {
        view: SweeperWidget::new(root_path.clone(), crew, CrewOptions::default()),
        show_no_selection: false,
        dry_run: args.dry_run,
    };

    loop {
        terminal.draw(|frame| {
            if args.ui_logger {
                let layout =
                    Layout::horizontal(&[Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(frame.size());

                frame.render_widget(&view, layout[0]);
                frame.render_widget(TuiAppLoggerWidget, layout[1]);
            } else {
                frame.render_widget(&view, frame.size());
            }
        })?;

        view.poll();

        if event::poll(std::time::Duration::from_millis(16))? {
            let event = event::read()?;
            view.handle_event(&event);
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
