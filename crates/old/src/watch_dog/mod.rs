use regex::Regex;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// scan desktop for new ppt files
pub fn file_moniter(
    path: &Path,
    exts: &[String],
    pattern: Option<&str>,
    min_depth: Option<usize>,
    max_depth: Option<usize>,
) -> Vec<PathBuf> {
    log::info!("Start scanning {}", path.display());

    let mut selected_files: Vec<PathBuf> = vec![];

    let walker = {
        let mut temp_walkdir = WalkDir::new(path);
        if let Some(min_depth) = min_depth {
            temp_walkdir = temp_walkdir.min_depth(min_depth);
        };
        if let Some(max_depth) = max_depth {
            temp_walkdir = temp_walkdir.max_depth(max_depth);
        };
        temp_walkdir.into_iter()
    };

    //for entry in path.read_dir().expect("read_dir call failed") {

    for entry in walker
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
    {
        // exclude hidden files
        // let entry = entry.expect("read_dir yielded error");
        let path = entry.into_path();

        if path.is_file() {
            // exclude temp files
            if path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("~$")
            {
                log::trace!("Skipping temp file {}", path.display());
                continue;
            }
            log::trace!("{} is not temp file", path.display());
            // TODO: use customised extensions instead of hardcoding
            if let Some(ext) = path.extension() {
                let ext_string = ext.to_str().unwrap().to_lowercase();
                if exts.contains(&ext_string) {
                    log::trace!("Found {}", path.display());
                    selected_files.push(path);
                    continue;
                }
            }
            if let Some(pattern) = pattern {
                log::trace!("Checking {} against {}", path.display(), pattern);
                let re = match Regex::new(pattern) {
                    Ok(re) => re,
                    Err(_) => {
                        log::error!("Invalid regex pattern: {}", pattern);
                        continue;
                    }
                };
                if re.is_match(path.file_name().unwrap().to_str().unwrap()) {
                    log::trace!("Found {}", path.display());
                    selected_files.push(path);
                }
            } else {
                log::trace!(
                    "Skipping file {}, because it is not our target.",
                    path.display()
                );
            }
        } else {
            log::trace!("Skipping directory {}", path.display());
        }
    }
    log::info!("Found {} files", selected_files.len());

    return selected_files;
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map_or(false, |s| s.starts_with('.'))
}

/// Get the sha256 hash of all files in a given list.
pub fn get_hashes<'a>(
    path_bufs: &[PathBuf],
) -> Result<HashMap<PathBuf, String>, Box<dyn std::error::Error>> {
    let mut map_of_hashes: std::collections::HashMap<PathBuf, String> = HashMap::new();

    for path in path_bufs.iter() {
        let hash = get_file_sha256(path)?;
        map_of_hashes.insert(path.clone(), hash);
    }

    return Ok(map_of_hashes);
}

/// Get the sha256 hash of a given file.
pub fn get_file_sha256(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
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

    return Ok(result);
}

/** Compare SHA256 hashes of files in two HashMaps,
and return a vector of files that altered. */
pub fn get_changed_files(
    old_map: &HashMap<PathBuf, String>,
    new_map: &HashMap<PathBuf, String>,
) -> Vec<PathBuf> {
    let mut changed_files: Vec<PathBuf> = Vec::new();

    for (path, new_hash) in new_map.iter() {
        match old_map.get(path) {
            Some(old_hash) => {
                if old_hash != new_hash {
                    changed_files.push(path.to_path_buf());
                }
            }
            None => {
                changed_files.push(path.to_path_buf());
            }
        }
    }
    changed_files
}
