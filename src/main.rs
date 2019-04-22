use std::path::PathBuf;

fn run_clippy() {
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
    //println!("crate_dir: {}, cargo_target_dir {}",Crate_dir, target_dir.display());
    //println!("output: {:?}", CLIPPY);
    let stderr = String::from_utf8_lossy(&clippy.stderr).to_string();
    let stdout = String::from_utf8_lossy(&clippy.stdout).to_string(); // json

    let mut results = Vec::new();

    stdout.lines().for_each(|line| {
        if line.starts_with("error: internal compiler error:")
            || line.starts_with("query stack during panic:")
        {
            results.push(format!("ERROR:   {}", line));
        }
    });
    stderr.lines().for_each(|line| {
        if line.starts_with("error: internal compiler error:")
            || line.starts_with("query stack during panic:")
        {
            results.push(format!("ERROR:   {}", line));
        }
    });

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

    let mut ids = Vec::new();
    results
        .iter()
        .for_each(|line| ids.push(line.split(' ').last().unwrap()));

    let mut summary: Vec<(usize, &&str)> = ids
        .iter()
        .map(|id_outer| (ids.iter().filter(|id| id == &id_outer).count(), id_outer))
        .collect::<Vec<_>>();

    summary.sort();
    summary.dedup();

    println!("\n\nSummary\n");

    for i in summary {
        let (numb, id) = i;
        println!("{}, {}", numb, id);
    }
}

#[derive(Debug, Clone)]
struct Crat {
    name: &'static str,
    version: semver::Version,
}

impl Crat {
    fn new(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version: semver::Version::parse(version).unwrap(),
        }
    }
}

fn download_crate(krate: Crat) {
    println!("Downloading {}-{} ...", krate.name, krate.version);
    let mut url: String = String::from("https://crates.io/api/v1/crates/");
    url.push_str(krate.name);
    url.push_str("/");
    url.push_str(&krate.version.to_string());
    url.push_str("/");
    url.push_str("download");

    let mut req =
        reqwest::get(url.as_str()).expect(&format!("Failed to downloadCrate {:?}", krate));
    let filename = format!("{}-{}.crate", krate.name, krate.version.to_string());
    let dest_path = PathBuf::from("downloads/").join(filename);
    let mut dest_file = std::fs::File::create(&dest_path).unwrap();

    std::io::copy(&mut req, &mut dest_file).unwrap();
}

fn extract_crate(path: PathBuf) {
    let krate = std::fs::File::open(path);
}

fn main() {
    let cargo = Crat::new("cargo", "0.35.0");
    let cargo_old = Crat::new("cargo", "0.34.0");

    // create a download dir
    let download_dir = PathBuf::from("downloads");
    if !download_dir.is_dir() {
        std::fs::create_dir(download_dir).unwrap();
    }

    download_crate(cargo);
    download_crate(cargo_old);
}

/*
let refresh_rate = std::time::Duration::from_secs(10);

println!("getting: {}", url);

let mut content_old = String::new();

loop {
    let mut req = reqwest::get(url).expect("Could not get url");
    let content = req.text().unwrap();

    let line_diff = content.lines().count() - content_old.lines().count();
    let old_lines = content.lines().count() - line_diff;
    // only print new lines (skip number of previous lines)
    content
        .lines()
        .skip(old_lines)
        .for_each(|line| println!("{}", line));

    std::thread::sleep(refresh_rate);
    content_old = content;
}
*/
