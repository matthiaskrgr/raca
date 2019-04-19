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
    let stdout = String::from_utf8_lossy(&clippy.stdout).to_string(); // json

    let mut results = Vec::new();

    stdout
        .lines()
        .map(|raw_line| serde_json::from_str(raw_line).unwrap())
        .map(|x: serde_json::Value| x)
        .filter(|x| x["reason"] == "compiler-message")
        .for_each(|json| {
            let pid = &json["package_id"].to_string();

            let pkg = pid
                .split_whitespace()
                .nth(0)
                .unwrap()
                .trim_matches('\"')
                .to_string();
            let version = pid.split_whitespace().nth(1).unwrap();

            // HACK
            let srcs: Vec<serde_json::Value> =
                serde_json::from_str(&json["message"]["spans"].to_string()).unwrap();

            let mut code_locs = String::new();
            for i in &srcs {
                let name = i["file_name"].to_string().trim_matches('\"').to_string();
                let lstart = &i["line_start"];
                let lend = &i["line_end"];
                let cstart = &i["column_start"];
                let cend = &i["column_start"];
                if lstart == lend && cstart == cend {
                    code_locs.push_str(&format!("{}:{}:{}", name, lstart, cstart));
                } else {
                    code_locs.push_str(&format!(
                        "{}:{}:{} -> {}:{}:{}",
                        name, lstart, cstart, name, lend, cend
                    ));
                }
            }

            let id = &json["message"]["code"]["code"]
                .to_string()
                .trim_matches('\"')
                .to_string();

            let msg = format!("{} {} {} {}", pkg, version, code_locs, id);
            //println!("{}", msg);
            results.push(msg);
        });

    results.sort();
    results.dedup();

    results.iter().for_each(|x| println!("{}", x));

    //println!("err: {}", stderr);
    //println!("out: {}", stdout);

    if stderr.starts_with("error: internal compiler error:")
        || stderr.starts_with("query stack during panic:")
        || stdout.starts_with("error: internal compiler error:")
        || stdout.starts_with("query stack during panic:")
    {
        println!(" ERROR: something crashed");
    } else {
        // println!(" ok");
    }
}
