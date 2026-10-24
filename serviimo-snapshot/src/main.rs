use std::path::PathBuf;

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let command = arguments.next();
    let result = match command.as_deref() {
        Some(command) if command == std::ffi::OsStr::new("inspect") => {
            let path = arguments.next().map(PathBuf::from);
            if path.is_none() || arguments.next().is_some() {
                usage();
            }
            serviimo_snapshot::inspect(&path.unwrap())
                .map(|report| serde_json::to_string_pretty(&report).unwrap())
        }
        Some(command) if command == std::ffi::OsStr::new("compare") => {
            let before = arguments.next().map(PathBuf::from);
            let after = arguments.next().map(PathBuf::from);
            if before.is_none() || after.is_none() || arguments.next().is_some() {
                usage();
            }
            serviimo_snapshot::compare(&before.unwrap(), &after.unwrap())
                .map(|report| serde_json::to_string_pretty(&report).unwrap())
        }
        _ => usage(),
    };
    match result {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("snapshot operation failed: {error}");
            std::process::exit(1);
        }
    }
}

fn usage() -> ! {
    eprintln!("usage: serviimo-snapshot inspect <local-file>");
    eprintln!("       serviimo-snapshot compare <before-file> <after-file>");
    std::process::exit(2);
}
