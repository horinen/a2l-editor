use anyhow::Result;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const SAMPLE_SIZE: usize = 1024 * 1024;

pub fn compute_file_hash(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();
    let modified_time = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let mut file = File::open(path)?;

    let mut hasher = Sha256::new();

    let sample_size = SAMPLE_SIZE.min(file_size as usize);
    let mut buffer = vec![0u8; sample_size];
    file.read_exact(&mut buffer)?;
    hasher.update(&buffer);

    hasher.update(file_size.to_le_bytes());
    hasher.update(modified_time.to_le_bytes());

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(1024), "1.00 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
    }
}
