use std::{collections::HashMap, path::{Path, PathBuf}};
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

            // exclude temp files
            if path.file_name().unwrap().to_str().unwrap().starts_with("~$") {
                continue;
            }

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
                    _ => {continue;}
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

/// Get the sha256 hash of all files in a given list.
pub fn get_hashes<'a>(path_bufs: &[PathBuf]) -> Result<HashMap<PathBuf, String>, Box<dyn std::error::Error>> {
    let mut map_of_hashes: std::collections::HashMap<PathBuf, String> = HashMap::new();

    for path in path_bufs.iter() {
        let hash = get_file_sha256(path)?;
        map_of_hashes.insert(path.clone(), hash);
    }

    return Ok(map_of_hashes);
}

/// Get the sha256 hash of a given file.
fn get_file_sha256(path: &Path) -> Result<String, Box<dyn std::error::Error>> {

    log::debug!("Getting sha256 hash of {}", path.display());
    
    // get file reader
    log::trace!("Opening file {}", path.display());
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    // create a hasher instance
    log::trace!("Creating hasher instance for {}", path.display());
    let mut hasher = Sha256::new();

    // read the file contents into the hasher
    log::trace!("Reading file contents of {}", path.display());
    std::io::copy(&mut reader, &mut hasher)?;

    // get the final hash value
    log::trace!("Getting final hash value of {}", path.display());
    let result = hasher.finalize();
    let result = format!("{:x}", result);

    log::trace!("Hash of {} is {}", path.display(), result);
    
    return Ok(result)
}


/** Compare SHA256 hashes of files in two HashMaps, 
    and return a vector of files that altered. */
pub fn get_changed_files(
    old_map: &HashMap<PathBuf, String>, new_map: &HashMap<PathBuf, String>
) -> Vec<PathBuf> {

    let mut changed_files: Vec<PathBuf> = Vec::new();

    for (path, new_hash) in new_map.iter() {
        match old_map.get(path) {
            Some(old_hash) => {
                if old_hash != new_hash {
                    changed_files.push(path.to_path_buf());
                }
            },
            None => {
                changed_files.push(path.to_path_buf());
            }
        }
    }
    changed_files
}
