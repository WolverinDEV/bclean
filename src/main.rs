#![feature(btree_extract_if)]

use std::{
    self,
    env,
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
    self,
    event::{
        self,
        KeyCode,
        KeyEventKind,
    },
};
use ratatui::{
    self,
    layout::{
        Constraint,
        Layout,
    },
};
use tui_logger::Drain;
use ui::{
    AppView,
    TuiAppLoggerWidget,
};

mod term;
mod ui;
mod utils;

/// Automate the cleanup of left over build files
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Specify the root directory where bclean should search for sweepable targets.
    /// Note: This can be a relative path.
    #[arg(short, long, verbatim_doc_comment)]
    root: Option<PathBuf>,

    /// Display the log in the terminal as a split screen.
    #[arg(long)]
    ui_logger: bool,

    /// Do not actually sweep anything. Just simulate it.
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

    let mut terminal = term::setup()?;
    terminal.clear()?;

    let crew = {
        let mut crew = SweeperCrew::new();
        crew.register(RustSweeper::new());
        crew.register(NodeSweeper::new());
        crew.register(CMakeSweeper::new());

        crew
    };

    let mut app_view = AppView::new(
        root_path.clone(),
        crew,
        CrewOptions::default(),
        args.dry_run,
    );

    loop {
        terminal.draw(|frame| {
            if args.ui_logger {
                let layout =
                    Layout::horizontal(&[Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(frame.size());

                frame.render_widget(&app_view, layout[0]);
                frame.render_widget(TuiAppLoggerWidget, layout[1]);
            } else {
                frame.render_widget(&app_view, frame.size());
            }
        })?;

        app_view.poll();

        if event::poll(std::time::Duration::from_millis(16))? {
            let event = event::read()?;
            app_view.handle_event(&event);
            if let event::Event::Key(key) = event {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }
    }

    Ok(())
}
