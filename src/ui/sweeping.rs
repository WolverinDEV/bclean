use bclean::SweepableTarget;
use crossterm::event::Event;
use ratatui::{
    text::Line,
    widgets::Widget,
};

pub struct SweepingWidget {
    targets: Vec<Box<dyn SweepableTarget>>,
}

impl SweepingWidget {
    pub fn new(targets: Vec<Box<dyn SweepableTarget>>) -> Self {
        Self { targets }
    }

    pub fn poll(&mut self) {}

    pub fn handle_event(&mut self, _event: &Event) {}
}

impl Widget for &SweepingWidget {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        Line::raw(format!("Sweeping {} targets", self.targets.len())).render(area, buf)
    }
}
