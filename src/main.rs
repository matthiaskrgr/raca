use std::io::Write;
use std::path::PathBuf;

fn run_clippy(path: PathBuf) -> Vec<CheckResult> {
    println!("Checking {} ...", &path.display());
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
        .current_dir(&path)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&clippy.stderr).to_string();
    let stdout = String::from_utf8_lossy(&clippy.stdout).to_string(); // json

    let mut results = Vec::new();

    /*
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
    */

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

            let mut code_locs = Vec::new();
            for i in &srcs {
                let name = i["file_name"].to_string().trim_matches('\"').to_string();
                let lstart = &i["line_start"];
                let lend = &i["line_end"];
                let cstart = &i["column_start"];
                let cend = &i["column_start"];
                if lstart == lend && cstart == cend {
                    let loc1 = SrcLoc {
                        file: name,
                        line: lstart.to_string().parse::<u32>().unwrap(),
                        column: cstart.to_string().parse::<u32>().unwrap(),
                    };
                    code_locs.push(loc1);
                } else {
                    let loc1 = SrcLoc {
                        file: name.clone(),
                        line: lstart.to_string().parse::<u32>().unwrap(),
                        column: cstart.to_string().parse::<u32>().unwrap(),
                    };

                    let loc2 = SrcLoc {
                        file: name,
                        line: lend.to_string().parse::<u32>().unwrap(),
                        column: cend.to_string().parse::<u32>().unwrap(),
                    };
                    code_locs.push(loc1);
                    code_locs.push(loc2);
                }
            }

            let id = &json["message"]["code"]["code"]
                .to_string()
                .trim_matches('\"')
                .to_string();

            let chkrslt = CheckResult {
                krate: pkg.to_string(),
                version: version.to_string(),
                id: id.to_string(),
                src_locs: code_locs,
            };

            results.push(chkrslt);
        });

    results.sort_by_key(|chrs| format!("{:?}", chrs));
    results.dedup_by_key(|chrs| format!("{:?}", chrs));

    //    results
    //    .iter()
    //     .for_each(|result| println!("{}", result.pretty()));

    let mut ids = Vec::new();
    results
        .iter()
        .for_each(|result| ids.push(result.id.clone()));

    let mut summary: Vec<(usize, &String)> = ids
        .iter()
        .map(|id_outer| (ids.iter().filter(|id| id == &id_outer).count(), id_outer))
        .collect::<Vec<_>>();

    summary.sort();
    summary.dedup();

    println!("\nSummary: {}", &path.display());

    for i in summary {
        let (numb, id) = i;
        println!("{}, {}", numb, id);
    }
    results
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

#[derive(Debug, Clone)]
struct CheckResult {
    krate: String,
    version: String,
    id: String,
    src_locs: Vec<SrcLoc>, // source code locations
}

impl CheckResult {
    fn pretty(&self) -> String {
        let locstr = {
            if self.src_locs.len() == 2 {
                format!(
                    "{}->{}",
                    self.src_locs[0].pretty(),
                    self.src_locs[1].pretty()
                )
            } else {
                // 1
                self.src_locs[0].pretty()
            }
        };
        format!("{}-{} {} {}", self.krate, self.version, self.id, locstr)
    }
}

#[derive(Debug, Clone)]
struct SrcLoc {
    // source code location
    file: String,
    line: u32,
    column: u32,
}

impl SrcLoc {
    fn pretty(&self) -> String {
        format!("{}:{}:{}", self.file, self.line, self.column)
    }
}

fn download_crate(krate: &Crat) -> PathBuf {
    println!("Downloading {}-{} ...", krate.name, krate.version);
    let mut url: String = String::from("https://crates.io/api/v1/crates/");
    url.push_str(krate.name);
    url.push_str("/");
    url.push_str(&krate.version.to_string());
    url.push_str("/");
    url.push_str("download");

    let mut req = reqwest::get(url.as_str())
        .unwrap_or_else(|_| panic!("Failed to downloadCrate {:?}", krate));
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

fn process_logs(check_results: Vec<CheckResult>, kratename: String) {
    let log_dir = PathBuf::from("logs");

    let filename = log_dir.join(kratename);
    let mut output = std::fs::File::create(filename).unwrap();

    check_results.iter().for_each(|line| {
        let mut line_with_lf = line.pretty();
        line_with_lf.push_str("\n");

        output.write_all(line_with_lf.as_bytes()).unwrap()
    });
}

fn main() {
    let krates = vec![
        Crat::new("cargo", "0.35.0"),
        Crat::new("crossbeam-utils", "0.6.5"),
        Crat::new("mdbook", "0.2.3"),
        Crat::new("parking_lot", "0.7.1"),
        Crat::new("quote", "0.6.12"),
        Crat::new("ryu", "0.2.7"),
        Crat::new("serde", "1.0.90"),
        Crat::new("syn", "0.15.32"),
        Crat::new("thread_local", "0.3.6"),
        Crat::new("tokei", "9.1.1"),
        Crat::new("unicode-normalization", "0.1.8"),
        Crat::new("winapi", "0.3.7"),
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

    let log_dir = PathBuf::from("logs");
    if !log_dir.is_dir() {
        std::fs::create_dir(&log_dir).unwrap();
    }

    // download and extract all crates
    for k in &krates {
        let dest_file = download_crate(k);
        extract_crate(dest_file, archives_dir.clone());
    }

    // start checking crates via clippy nad process the logs
    for (i, k) in std::fs::read_dir(archives_dir.clone()).unwrap().enumerate() {
        println!("i {}, k {:?}", i, k);
        let results = run_clippy(k.unwrap().path());

        let kratename = format!("{}-{}", &krates[i].name, &krates[i].version);
        process_logs(results, kratename);
    }
}
