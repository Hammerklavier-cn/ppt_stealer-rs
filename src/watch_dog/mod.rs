use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use sha2::{Sha256, Digest};

/// scan desktop for new ppt files
pub fn file_moniter(path: &Path) -> Vec<PathBuf> {

    log::info!("Start scanning {}", path.display());

    let mut document_files: Vec<PathBuf> = vec![];

    let walker = WalkDir::new(path)
        .into_iter();
        

    //for entry in path.read_dir().expect("read_dir call failed") {
    
    for entry in walker
                            .filter_entry(|e| !is_hidden(e))
                            .filter_map(|e| e.ok()) { // exclude hidden files
        // let entry = entry.expect("read_dir yielded error");
        let path = entry.into_path();

        if path.is_file() {

            if let Some(ext) = path.extension() {
                let ext_str = ext.to_str().unwrap().to_lowercase();
                match ext_str.as_str() {
                    "ppt" | "pptx" | "odp" => {
                        log::trace!("Found powerpoint/impress file {}", path.display());
                        document_files.push(path);
                    }
                    "doc" | "docx" | "odt" => {
                        log::trace!("Found document/writer file {}", path.display());
                        document_files.push(path);
                    }
                    "xls" | "xlsx" | "ods" => {
                        log::trace!("Found excel/calc file {}", path.display());
                        document_files.push(path);
                    }
                    "pdf" => {
                        log::trace!("Found pdf file {}", path.display());
                        document_files.push(path);
                    }
                    _ => {}
                }
            }
        }
    }
    log::debug!("Found {} files", document_files.len());

    return document_files;
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map_or(false, |s| s.starts_with('.'))
}

/// Get the sha256 hash of a given file.
pub fn get_file_sha256(path: &Path) -> String {

    log::debug!("Getting sha256 hash of {}", path.display());
    
    // get file reader
    log::trace!("Opening file {}", path.display());
    let file = std::fs::File::open(path).expect("Failed to open file");
    let mut reader = std::io::BufReader::new(file);

    // create a hasher instance
    log::trace!("Creating hasher instance for {}", path.display());
    let mut hasher = sha2::Sha256::new();

    // read the file contents into the hasher
    log::trace!("Reading file contents of {}", path.display());
    std::io::copy(&mut reader, &mut hasher).expect("Failed to read file");

    // get the final hash value
    log::trace!("Getting final hash value of {}", path.display());
    let result = hasher.finalize();
    let result = format!("{:x}", result);

    log::debug!("Hash of {} is {}", path.display(), result);
    return result;
}