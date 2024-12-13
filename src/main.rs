use clap::{Parser, ArgGroup};
use log;
use env_logger;

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

    println!("Hello, world!");
}
