use flate2::{Compression, write::GzEncoder};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs,
    io::{self, BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
};


const GIT_VERSION: &str = git_version::git_version!();

const BINARY_TARGETS: [&'static str; 6] = [
    "tagsave",
    "tagview",
    "streamer",
    "tcat",
    "txt2tags",
    "checkrun",
];

// Executables that statically link proprietary vendor code
const WINDOWS_NONDISTRIBUTABLE: [&'static str; 1] = [
    "streamer",
];

type DynError = Box<dyn std::error::Error>;

fn main() -> Result<(), DynError> {
    let task = env::args().nth(1);
    match task.as_ref().map(|it| it.as_str()) {
        Some("dist") => dist()?,
        _ => help(),
    }
    Ok(())
}

fn help() {
    eprintln!(
        r#"Tasks:

dist    build dist artifacts and package
    "#)
}

fn dist() -> Result<(), DynError> {
    let _ = fs::remove_dir_all(&dist_tmp_dir());
    fs::create_dir_all(&dist_tmp_dir())?;
    fs::create_dir_all(&dist_dir())?;

    dist_binary()?;

    Ok(())
}

fn dist_binary() -> Result<(), DynError> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    if cfg!(windows) {
        let build_status = Command::new(cargo)
            .current_dir(project_root())
            .args(&["build", "--release"])
            .status()?;
        if !build_status.success() {
            panic!("cargo build failed")
        }
    } else if cfg!(linux) {
        let build_status = Command::new(cargo)
            .env("CXX", "gcc-10")
            .current_dir(project_root())
            .args(&["build", "--release"])
            .status()?;
        if !build_status.success() {
            panic!("cargo build failed")
        }
    }
    let target =
        if cfg!(windows) {
            "x86_64-pc-windows-msvc"
        } else if cfg!(linux) {
            "x86_64-unknown-linux-gnu"
        } else {
            panic!("dist supports only windows and linux targets")
    };
    for binary in BINARY_TARGETS {
        let mut bin = project_root().join(format!("target/release/{}", binary));
        let mut dst = dist_tmp_dir().join(format!("{}", binary));
        if cfg!(windows) {
            bin.set_extension("exe");
            dst.set_extension("exe");
        }
        fs::copy(&bin, &dst)
            .expect(&format!("cannot find {}", binary));
        println!("{} copied to distdir", binary);
    }
    let filename = format!(
        "tagger-{}-{}{}.tar.gz",
        GIT_VERSION,
        target,
        if cfg!(windows) { "-NO_DISTRIBUTE" } else { "" },
    );
    let filepath = dist_dir().join(&filename);
    let sha256path = dist_dir().join("SHA256");
    {
        let tar_gz = fs::File::create(&filepath)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);
        tar.append_dir_all("tagger", dist_tmp_dir())?;
        tar.finish()?;
    }
    println!("tarball prepared");
    {
        let mut tar_gz = fs::File::open(&filepath)?;
        let mut sha256 = Sha256::new();
        io::copy(&mut tar_gz, &mut sha256)?;
        let raw_checksum = sha256.finalize();
        let checksum = format!("{:x}", raw_checksum);
        let mut all_checksums = String::new();
        if let Ok(sha256file) = fs::File::open(&sha256path) {
            let buf = BufReader::new(sha256file);
            for line in buf.lines() {
                let text = line?;
                // Retain existing tarball checksums other than for our target
                if !text.contains(&filename) {
                    all_checksums.push_str(&text);
                    all_checksums.push('\n');
                }
            }
        }
        all_checksums.push_str(&checksum);
        all_checksums.push_str(&"  ");
        all_checksums.push_str(&filename);
        all_checksums.push('\n');
        if let Ok(sha256file) = fs::File::create(sha256path) {
            let mut buf = BufWriter::new(sha256file);
            buf.write_all(&all_checksums.as_bytes())?;
            buf.flush()?;
        }
    }
    println!("SHA256 checksum prepared");
    if cfg!(windows) {
        println!("Windows: preparing separate redistributable release");
        // Remove nonredistributable binaries
        for binary in WINDOWS_NONDISTRIBUTABLE {
            let mut bin = dist_tmp_dir().join(format!("{}", binary));
            bin.set_extension("exe");
            fs::remove_file(bin)?;
        }
        let filename = format!(
            "tagger-{}-{}.tar.gz",
            GIT_VERSION,
            target,
        );
        let filepath = dist_dir().join(&filename);
        let sha256path = dist_dir().join("SHA256");
        {
            let tar_gz = fs::File::create(&filepath)?;
            let enc = GzEncoder::new(tar_gz, Compression::default());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all("tagger", dist_tmp_dir())?;
            tar.finish()?;
        }
        {
            let mut tar_gz = fs::File::open(&filepath)?;
            let mut sha256 = Sha256::new();
            io::copy(&mut tar_gz, &mut sha256)?;
            let raw_checksum = sha256.finalize();
            let checksum = format!("{:x}", raw_checksum);
            let mut all_checksums = String::new();
            if let Ok(sha256file) = fs::File::open(&sha256path) {
                let buf = BufReader::new(sha256file);
                for line in buf.lines() {
                    let text = line?;
                    // Retain existing tarball checksums other than for our target
                    if !text.contains(&filename) {
                        all_checksums.push_str(&text);
                        all_checksums.push('\n');
                    }
                }
            }
            all_checksums.push_str(&checksum);
            all_checksums.push_str(&"  ");
            all_checksums.push_str(&filename);
            all_checksums.push('\n');
            if let Ok(sha256file) = fs::File::create(sha256path) {
                let mut buf = BufWriter::new(sha256file);
                buf.write_all(&all_checksums.as_bytes())?;
                buf.flush()?;
            }
        }
    }
    fs::remove_dir_all(dist_tmp_dir())?;
    println!("dist_tmp dir cleanup");
    Ok(())
}

fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .expect("cannot find project root")
        .to_path_buf()
}

fn dist_tmp_dir() -> PathBuf {
    project_root().join("target/dist_tmp")
}

fn dist_dir() -> PathBuf {
    project_root().join("target/dist")
}