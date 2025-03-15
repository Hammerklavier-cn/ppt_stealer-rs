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

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the slint GUI application.
    Gui,
    /// Start the command-line interface.

    #[command(group(
        ArgGroup::new("auth")
            .args(&["password", "key_auth"])
            .required(false)
            .multiple(false)
    ))]
    NoGui {
        #[arg(short = 'i', long, help = "SSH IP address or domain")]
        ip: String,

        #[arg(short = 'p', long, help = "SSH IP port", default_value_t = 22)]
        port: i64,

        #[arg(short = 'u', long, help = "SSH username")]
        username: String,

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

        #[arg(long, help = "Scan additional folder for files.")]
        remote_folder_name: Option<String>,

        #[arg(long, help = "Scan USB for files.")]
        usb: bool,

        #[command(flatten)]
        scan_params: ScanParams,
    },
}

#[derive(Args, Debug, Clone)]
#[group(required = false, multiple = true)]
pub struct ScanParams {
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

/// This is a shared function for debugging purposes.
pub fn shared_function() {
    println!("You successfully called the shared function!")
}

pub fn get_args() -> Cli {
    Cli::parse()
}
