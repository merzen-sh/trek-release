use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::FileOptions;

#[derive(Parser)]
#[command(name = "trek-release")]
#[command(about = "A release management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Package a FiveM resource for release
    Pack {
        /// Input resource directory
        #[arg(short, long)]
        input: String,

        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Package name
        #[arg(short, long)]
        name: String,

        /// Package version
        #[arg(short, long)]
        version: String,

        /// Print a markdown summary of the packed files
        #[arg(short, long)]
        summary: bool,

        /// Print files that would be packed without creating the zip
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Pack {
            input,
            output,
            name,
            version,
            summary,
            dry_run,
        } => {
            let input_path = Path::new(input);
            let output_dir = Path::new(output);

            let zip_filename = format!("{}-{}.zip", name, version);
            let zip_path = output_dir.join(&zip_filename);

            let matcher = TrekPackMatcher::new(input_path);

            let mut files_to_pack: Vec<(PathBuf, u64)> = Vec::new();

            for entry in WalkDir::new(input_path)
                .into_iter()
                .filter_entry(|e| !e.file_name().to_str().is_some_and(|s| s == ".trek-pack"))
            {
                let entry = entry.expect("Failed to read entry");
                if entry.file_type().is_file() {
                    let relative_path = entry.path().strip_prefix(input_path).unwrap();
                    if matcher.matches(relative_path) {
                        let size = entry.metadata().ok().map(|m| m.len()).unwrap_or(0);
                        files_to_pack.push((entry.path().to_path_buf(), size));
                    }
                }
            }

            if *dry_run {
                print_dry_run(name, version, &zip_path, &files_to_pack, &input_path);
                return;
            }

            if let Some(parent) = zip_path.parent() {
                fs::create_dir_all(parent).expect("Failed to create output directory");
            }

            let file = File::create(&zip_path).expect("Failed to create zip file");
            let mut zip = ZipWriter::new(file);

            let options: FileOptions<'_, ()> =
                FileOptions::default().compression_method(CompressionMethod::Deflated);

            for (file_path, _size) in &files_to_pack {
                let relative_path = file_path.strip_prefix(input_path).unwrap();
                let relative_str = relative_path.to_str().unwrap();

                zip.start_file(relative_str, options.clone())
                    .expect("Failed to start zip entry");

                let mut f = File::open(file_path).expect("Failed to open file");
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer).expect("Failed to read file");
                zip.write_all(&buffer).expect("Failed to write to zip");
            }

            zip.finish().expect("Failed to finalize zip file");

            if *summary {
                print_summary(name, version, &zip_path, &files_to_pack, &input_path);
            } else {
                let total_size: u64 = files_to_pack.iter().map(|(_, s)| s).sum();
                println!(
                    "Created: {} ({} files, {})",
                    zip_path.display(),
                    files_to_pack.len(),
                    format_size(total_size)
                );
            }
        }
    }
}

struct TrekPackMatcher {
    include: GlobSet,
    exclude: GlobSet,
    has_patterns: bool,
}

impl TrekPackMatcher {
    fn new(input_dir: &Path) -> Self {
        let trek_pack_path = input_dir.join(".trek-pack");
        let mut include_builder = GlobSetBuilder::new();
        let mut exclude_builder = GlobSetBuilder::new();
        let mut has_patterns = false;

        if let Ok(content) = std::fs::read_to_string(&trek_pack_path) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(pattern) = line.strip_prefix('!') {
                    if let Ok(glob) = Glob::new(pattern) {
                        exclude_builder.add(glob);
                    }
                } else if let Ok(glob) = Glob::new(line) {
                    include_builder.add(glob);
                    has_patterns = true;
                }
            }
        }

        Self {
            include: include_builder
                .build()
                .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap()),
            exclude: exclude_builder
                .build()
                .unwrap_or_else(|_| GlobSetBuilder::new().build().unwrap()),
            has_patterns,
        }
    }

    fn matches(&self, path: &Path) -> bool {
        if !self.has_patterns {
            return true;
        }
        if self.exclude.is_match(path) {
            return false;
        }
        self.include.is_match(path)
    }
}

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = size as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

fn print_file_table(
    title: &str,
    extra_label: &str,
    name: &str,
    version: &str,
    zip_path: &Path,
    files: &[(PathBuf, u64)],
    input_path: &Path,
) {
    let total_size: u64 = files.iter().map(|(_, s)| s).sum();
    println!("# {title}");
    println!();
    println!("|Key|Value|");
    println!("|---|---|");
    println!("| **Name** | {name} |");
    println!("| **Version** | {version} |");
    println!("| **{extra_label}** | `{}` |", zip_path.display());
    println!("| **Total files** | {} |", files.len());
    println!("| **Total size** | {} |", format_size(total_size));
    println!();
    println!("## Files");
    println!();
    println!("| File | Size |");
    println!("|---|---:|");
    for (path, size) in files {
        let relative = path.strip_prefix(input_path).unwrap().to_str().unwrap();
        println!("| `{relative}` | {} |", format_size(*size));
    }
}

fn print_dry_run(
    name: &str,
    version: &str,
    zip_path: &Path,
    files: &[(PathBuf, u64)],
    input_path: &Path,
) {
    print_file_table("Dry Run", "Would create", name, version, zip_path, files, input_path);
}

fn print_summary(
    name: &str,
    version: &str,
    zip_path: &Path,
    files: &[(PathBuf, u64)],
    input_path: &Path,
) {
    println!();
    print_file_table("Pack Summary", "Output", name, version, zip_path, files, input_path);
}
