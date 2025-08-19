use std::path::PathBuf;
use walkdir::WalkDir;

pub fn find_dds_files(input_dir: &std::path::Path) -> Vec<PathBuf> {
    WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension()
                   .and_then(|ext| ext.to_str())
                   .map(|ext| ext.to_lowercase() == "dds")
                   .unwrap_or(false) {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .collect()
}
