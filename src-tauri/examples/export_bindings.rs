use std::env;
use std::path::PathBuf;

fn main() {
    let mut arguments = env::args_os();
    let program = arguments.next().unwrap_or_else(|| "export_bindings".into());
    let Some(output) = arguments.next() else {
        eprintln!("usage: {} <output-path>", PathBuf::from(program).display());
        std::process::exit(2);
    };
    if arguments.next().is_some() {
        eprintln!("usage: {} <output-path>", PathBuf::from(program).display());
        std::process::exit(2);
    }

    if let Err(error) = isoboard_lib::contracts::export_typescript(PathBuf::from(output)) {
        eprintln!("could not export TypeScript IPC bindings: {error}");
        std::process::exit(1);
    }
}
