use std::fs;
use std::fs::{create_dir, read_dir};
use std::io::Write;
use std::process::Command;

fn main() {
    let clippy = std::process::Command::new("cargo")
        .arg("clippy")
        //    let clippy = std::process::Command::new(
        //        "/home/matthias/vcs/github/rust-clippy/target/debug/cargo-clippy",
        //    )
        .arg("--all-targets")
        .arg("--all-features")
        .arg("--message-format=json")
        .args(&[
            "--",
            "--cap-lints",
            "warn",
            "-Wclippy::internal",
            "-Wclippy::pedantic",
            "-Wclippy::nursery",
        ])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUST_BACKTRACE", "full")
        //            .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .unwrap();
    //println!("crate_dir: {}, cargo_target_dir {}", crate_dir, target_dir.display());
    //println!("output: {:?}", CLIPPY);
    let stderr = String::from_utf8_lossy(&clippy.stderr).to_string();
    let stdout = String::from_utf8_lossy(&clippy.stdout).to_string();
    if stderr.starts_with("error: internal compiler error:")
        || stderr.starts_with("query stack during panic:")
        || stdout.starts_with("error: internal compiler error:")
        || stdout.starts_with("query stack during panic:")
    {
        println!(" ERROR: something crashed");
    } else {
        println!(" ok");
    }
}
