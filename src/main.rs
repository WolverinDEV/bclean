use std::{
    self,
    env,
};

use args::{
    ArgSweeper,
    Args,
};
use bclean::{
    self,
    CrewOptions,
    SweeperCrew,
};
use clap::Parser;
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

mod args;
mod term;
mod ui;
mod utils;

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

        let mut sweepers = args.sweeper.clone();
        if !args.sweeper_no_defaults {
            sweepers.extend(ArgSweeper::default_configuration());
        }

        for (sweeper, options) in sweepers {
            log::info!("Register sweeper {:?} ({:?})", sweeper, options);
            crew.register_boxed(sweeper.create_from_options(options.as_ref().map(String::as_str))?);
        }
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
