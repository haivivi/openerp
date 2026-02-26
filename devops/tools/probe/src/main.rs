use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(err) = run() {
        eprintln!("probe collector failed: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os();
    let _bin = args.next();

    let out_dir = args
        .next()
        .ok_or("missing output directory argument")
        .map(PathBuf::from)?;

    let inputs: Vec<PathBuf> = args.map(PathBuf::from).collect();

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)?;
    }
    fs::create_dir_all(&out_dir)?;

    for (idx, src) in inputs.iter().enumerate() {
        let name = src
            .file_name()
            .unwrap_or_else(|| OsStr::new("artifact"))
            .to_string_lossy();
        let dst = out_dir.join(format!("{idx:04}_{name}"));
        copy_path(src, &dst)?;
    }

    eprintln!("Probe collected {} files", inputs.len());
    Ok(())
}

fn copy_path(src: &Path, dst: &Path) -> io::Result<()> {
    let metadata = fs::metadata(src)?;
    if metadata.is_dir() {
        copy_dir_recursive(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        copy_path(&src_path, &dst_path)?;
    }
    Ok(())
}
