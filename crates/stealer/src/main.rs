use cli::{get_args, shared_function, DebugLevel};

fn main() {
    println!("Hello, world!");
    shared_function();

    // parse command line arguments
    let args = get_args();

    // set up logging level
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
}
