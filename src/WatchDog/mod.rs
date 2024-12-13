use std::path::{Path, PathBuf};

// scan desktop for new ppt files
pub fn file_moniter(path: &Path) -> Vec<PathBuf> {

    log::info!("Start scanning {}", path.display());

    let mut ppt_files: Vec<PathBuf> = vec![];

    for entry in path.read_dir().expect("read_dir call failed") {
        let entry = entry.expect("read_dir yielded error");
        let path = entry.path();

        if path.is_file() {
            if path.extension().unwrap() == "ppt" {
                ppt_files.push(path);
            }
        }
    }

    return ppt_files;
}