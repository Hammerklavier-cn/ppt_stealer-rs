use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "ppt_stealer-rs", version)]
#[command(about, long_about = None, author)]
#[command(color = clap::ColorChoice::Always)]
#[command(help_template = "\
{bin} {version} by {author-with-newline}{about}

{usage-heading} {usage}

{all-args}

{after-help}")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(
        value_enum,
        short = 'L',
        long,
        next_line_help = true,
        help = "Debug level.",
        default_value_t = DebugLevel::Info)]
    pub debug_level: DebugLevel,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum DebugLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum UploadTarget {
    Local,
    SshServer,
    SmbServer,
    FtpServer,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the slint GUI application.
    Gui,
    /// Start the command-line interface.
    NoGui {
        #[command(flatten)]
        server_params: ServerParams,

        #[command(flatten)]
        target_params: TargetParams,

        #[command(flatten)]
        scan_params: ScanParams,
    },
}

#[derive(Args, Debug, Clone)]
#[command(group(
    ArgGroup::new("auth")
        .args(&["password", "key_auth"])
        .required(false)
        .multiple(false)
))]
#[group(required = false, multiple = true)]
pub struct ServerParams {
    #[arg(short = 'i', long, help = "Server IP address or domain")]
    pub ip: Option<String>,

    #[arg(short = 'p', long, help = "Service IP port")]
    pub port: Option<i64>,

    #[arg(short = 'u', long, help = "Service username")]
    pub username: Option<String>,

    #[arg(short = 'P', long, group = "auth", help = "Service password")]
    pub password: Option<String>,

    #[arg(
        long,
        default_value_t = false,
        group = "auth",
        next_line_help = true,
        help = "Use SSH key authentication. If not assigned, password authentication will be used."
    )]
    pub key_auth: bool,
}

#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct LocalParams {
    #[arg(long, help = "Local directory where you want to store the files.")]
    pub copy_to: String,
}

#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct TargetParams {
    #[arg(
        long,
        help = "Upload files to a ssh server. Note that you can only choose one kind of remote target! Only SshServer is implemented now.",
        default_value = "local",
        value_delimiter = ' '
    )]
    pub upload_targets: Vec<UploadTarget>,

    #[arg(long, help = "Scan additional folder for files.")]
    pub target_folder_name: Option<String>,
}

#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct ScanParams {
    #[arg(long, help = "Scan USB for files.")]
    pub usb: bool,

    #[arg(long, default_value_t = 30, help = "Refresh interval in seconds")]
    pub refresh_interval: u64,

    #[arg(long, help = "Custimised desktop path")]
    pub desktop_path: Option<String>,

    #[arg(long, short = 'm', help = "Minimum depth of file (included)")]
    pub min_depth: Option<usize>,

    #[arg(long, short = 'M', help = "Maximum depth of file (included)")]
    pub max_depth: Option<usize>,

    #[arg(long, short = 'a', help = "Additional paths to scan")]
    pub add_paths: Option<Vec<String>>,

    #[arg(long, short = 'r', help = "Regex pattern to match files")]
    pub regex: Option<String>,

    #[arg(
        long,
        help = "Assign file formats",
        default_value = "ppt pptx odp doc docx odt xls xlsx ods csv txt md",
        value_delimiter = ' '
    )]
    pub formats: Vec<String>,
}

/// This is a shared function for debugging purposes.
pub fn shared_function() {
    println!("You successfully called the shared function!")
}

pub fn get_args() -> Cli {
    Cli::parse()
}
