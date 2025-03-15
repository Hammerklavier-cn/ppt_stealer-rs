use cli::{get_args, shared_function};

fn main() {
    println!("Hello, world!");
    shared_function();

    let cli = get_args();
}
