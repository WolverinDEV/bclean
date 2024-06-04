use std::{
    io::{
        stdout,
        Stdout,
    },
    ops::{
        Deref,
        DerefMut,
    },
};

use crossterm::{
    terminal::{
        enable_raw_mode,
        EnterAlternateScreen,
    },
    ExecutableCommand,
};
use ratatui::{
    backend::{
        Backend,
        CrosstermBackend,
    },
    Terminal,
};

pub struct TerminalGuard<B: Backend> {
    inner: Terminal<B>,
}

impl<B: Backend> TerminalGuard<B> {
    pub fn setup(backend: B) -> anyhow::Result<Self> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;

        Ok(Self {
            inner: Terminal::new(backend)?,
        })
    }
}
impl<B: Backend> Drop for TerminalGuard<B> {
    fn drop(&mut self) {}
}
impl<B: Backend> Deref for TerminalGuard<B> {
    type Target = Terminal<B>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl<B: Backend> DerefMut for TerminalGuard<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub fn setup() -> anyhow::Result<TerminalGuard<CrosstermBackend<Stdout>>> {
    TerminalGuard::setup(CrosstermBackend::new(stdout()))
}
