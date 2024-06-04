use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{
        Block,
        Borders,
        Widget,
    },
};
use tui_logger::TuiLoggerWidget;

pub struct TuiAppLoggerWidget;

impl Widget for TuiAppLoggerWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let block = Block::new().title("Logging output").borders(Borders::LEFT);
        TuiLoggerWidget::default().block(block).render(area, buf);
    }
}
