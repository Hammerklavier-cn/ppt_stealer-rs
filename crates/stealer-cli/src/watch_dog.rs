use anyhow::{Result, anyhow};
use cli::ScanParams;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Get source directories according to scan parameters
#[allow(dead_code)]
pub(crate) fn get_source_directories(
    scan_params: &ScanParams,
) -> Result<BTreeSet<PathBuf>, anyhow::Error> {
    let mut source_pathbuf_set: BTreeSet<PathBuf> = BTreeSet::new();

    // Get local desktop path
    let desktop_path = match &scan_params.desktop_path {
        Some(path_str) => {
            let path = PathBuf::from(&path_str.trim()).canonicalize()?;
            match path.is_dir() {
                true => path,
                false => {
                    return Err(anyhow!(
                        "An invalid directory path is assigned! {}",
                        path_str
                    ));
                }
            }
        }
        None => {
            let desktop_path = match dirs::desktop_dir() {
                Some(path) => path,
                None => {
                    // return an error
                    return Err(anyhow!(
                        "Failed to verify path to `Desktop` of your computer. \
                        Please specify the desktop path manually through `--desktop-path`!"
                    ));
                }
            };
            desktop_path
        }
    };
    source_pathbuf_set.insert(desktop_path);

    // Verify usb paths
    if scan_params.usb {
        log::info!("`--usb` specified. Files from USB drives will be uploaded.");

        let mut disk_path_set: BTreeSet<PathBuf> = BTreeSet::new();

        let disks = sysinfo::Disks::new_with_refreshed_list();
        for disk in disks.into_iter().filter(|d| d.is_removable()) {
            disk_path_set.insert(disk.mount_point().to_path_buf());
        }

        source_pathbuf_set.extend(disk_path_set.into_iter());
    };

    // Verify additional paths
    if let Some(paths) = &scan_params.add_paths {
        log::info!("`--add-paths` specified. Files from these folders will be uploaded.");
        for path_str in paths.iter() {
            let path_buf = PathBuf::from(path_str.trim()).canonicalize()?;
            match path_buf.is_dir() {
                true => {
                    source_pathbuf_set.insert(path_buf);
                }
                false => {
                    return Err(anyhow!(
                        "An invalid directory path is assigned! {}",
                        path_str
                    ));
                }
            }
        }
    }

    Ok(source_pathbuf_set)
}
