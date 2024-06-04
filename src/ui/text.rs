use std::{
    borrow::Cow,
    time::Instant,
};

pub struct ScrollableText {
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
