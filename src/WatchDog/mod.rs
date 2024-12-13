use std::path::{Path, PathBuf};

// scan desktop for new ppt files
pub fn file_moniter(path: &Path) -> Vec<PathBuf> {

    log::info!("Start scanning {}", path.display());

    let mut document_files: Vec<PathBuf> = vec![];

    for entry in path.read_dir().expect("read_dir call failed") {
        let entry = entry.expect("read_dir yielded error");
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ["ppt", "pptx", "doc", "docx", "xls", "xlsx", "pdf"].contains(&ext.to_str().unwrap()) {
                    log::trace!("Found ppt file {}", path.display());
                    document_files.push(path);
                }
            }
        }
    }
    log::debug!("Found {} ppt files", document_files.len());

    return document_files;
}