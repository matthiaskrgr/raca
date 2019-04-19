fn main() {
    let clippy = std::process::Command::new("cargo")
        .arg("clippy")
        //    let clippy = std::process::Command::new(
        //        "/home/matthias/vcs/github/rust-clippy/target/debug/cargo-clippy",
        //    )
        // .arg("--all-targets")
        //.arg("--all-features")
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
    let stdout = String::from_utf8_lossy(&clippy.stdout).to_string(); // json

    stdout
        .lines()
        .map(|raw_line| serde_json::from_str(raw_line).unwrap())
        .map(|x: serde_json::Value| x)
        .filter(|x| x["reason"] == "compiler-message")
        .for_each(|json| {
            let pid = &json["package_id"].to_string();

            let pkg = pid.split_whitespace().nth(0).unwrap();
            let version = pid.split_whitespace().nth(1).unwrap();

            let src = &json["message"]["spans"][0]["file_name"];

            let id = &json["message"]["code"]["code"];

            let msg = format!("{} {} {} {}", pkg, version, id, src);
            println!("{}", msg);
        });

    //println!("err: {}", stderr);
    //println!("out: {}", stdout);

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
