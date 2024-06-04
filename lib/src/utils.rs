const SIZE_1KB: u64 = 1024;
const SIZE_1MB: u64 = 1024 * SIZE_1KB;
const SIZE_1GB: u64 = 1024 * SIZE_1MB;

pub fn format_file_size(size: u64) -> String {
    if size >= SIZE_1GB * 2 {
        format!("{:.2} GB", (size as f64) / (SIZE_1GB as f64))
    } else if size >= SIZE_1MB * 2 {
        format!("{:.2} MB", (size as f64) / (SIZE_1MB as f64))
    } else if size >= SIZE_1KB * 2 {
        format!("{:.2} KB", (size as f64) / (SIZE_1KB as f64))
    } else {
        format!("{} bytes", size)
    }
}
