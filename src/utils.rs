use std::time::Duration;

pub fn format_duration(value: &Duration) -> String {
    if value.as_secs() < 60 * 60 {
        format!(
            "{:0>2}:{:0>2}.{:0>2}",
            value.as_secs() / 60,
            value.as_secs() % 60,
            value.subsec_millis() / 10
        )
    } else {
        format!(
            "{:0>2}:{:0>2}:{:0>2}",
            value.as_secs() / (60 * 60),
            (value.as_secs() / 60) % 60,
            value.as_secs() % 60
        )
    }
}
