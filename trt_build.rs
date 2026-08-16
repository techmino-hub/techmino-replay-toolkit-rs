use std::process::Command;

fn main() {
    if option_env!("TRT_MARKERLESS_MODE").is_none() {
        assign_marker_value();
    }
}

fn assign_marker_value() {
    let output = match Command::new("git").args(["rev-parse", "HEAD"]).output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error launching git to infer commit hash: {e}");
            eprintln!("Are you sure git is installed and is in PATH?");

            std::process::exit(1);
        }
    };

    let git_hash = String::from_utf8_lossy(&output.stdout);

    let package_version = env!("CARGO_PKG_VERSION");

    println!("cargo:rustc-env=MARKER_VALUE='TRT v{package_version} ({git_hash})'");
}
