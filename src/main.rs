use std::path::PathBuf;

fn run_clippy(path: PathBuf) {
    println!("Checking {} ...", path.display());
    let clippy = std::process::Command::new("cargo")
        .arg("clippy")

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
        .current_dir(path)
        .output()
        .unwrap();

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

fn download_crate(krate: Crat) -> PathBuf {
    println!("Downloading {}-{} ...", krate.name, krate.version);
    let mut url: String = String::from("https://crates.io/api/v1/crates/");
    url.push_str(krate.name);
    url.push_str("/");
    url.push_str(&krate.version.to_string());
    url.push_str("/");
    url.push_str("download");

    let mut req =
        reqwest::get(url.as_str()).unwrap_or_else(|_| panic!("Failed to downloadCrate {:?}", krate));
    let filename = format!("{}-{}.crate", krate.name, krate.version.to_string());
    let dest_path = PathBuf::from("downloads/").join(filename);
    let mut dest_file = std::fs::File::create(&dest_path).unwrap();

    std::io::copy(&mut req, &mut dest_file).unwrap();
    dest_path
}

fn extract_crate(src_path: PathBuf, target_path: PathBuf) {
    println!(
        "Extracting {} into {}",
        src_path.display(),
        target_path.display()
    );
    let krate = std::fs::File::open(src_path).unwrap();
    let tar = flate2::read::GzDecoder::new(krate);
    let mut archiv = tar::Archive::new(tar);
    archiv.unpack(target_path).unwrap();
}

fn main() {
    let krates = vec![
        Crat::new("cargo", "0.35.0"),
        Crat::new("cargo", "0.34.0"),
        Crat::new("cargo", "0.33.0"),
    ];

    // create a download dir
    let download_dir = PathBuf::from("downloads");
    if !download_dir.is_dir() {
        std::fs::create_dir(download_dir).unwrap();
    }

    // create a archiv dir
    let archives_dir = PathBuf::from("archives");
    if !archives_dir.is_dir() {
        std::fs::create_dir(&archives_dir).unwrap();
    }

    // download and extract all crates
    for k in krates {
        let dest_file = download_crate(k);
        extract_crate(dest_file, archives_dir.clone());
    }

    // start checking crates via clippy
    for k in std::fs::read_dir(archives_dir.clone()).unwrap() {
        run_clippy(k.unwrap().path());
    }

}
