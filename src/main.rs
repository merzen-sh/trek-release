use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use clap::{Parser, Subcommand};
use flate2::write::GzEncoder;
use flate2::Compression;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

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
    },
}

fn display_files(input: &Path) {
    println!("\nFiles:");
    walk_dir(input, input, 0);
}

fn walk_dir(base: &Path, dir: &Path, depth: usize) {
    if let Ok(entries) = fs::read_dir(dir) {
        let mut entries: Vec<_> = entries
            .flatten()
            .filter(|e| e.file_name() != ".git")
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in &entries {
            let path = entry.path();
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let indent = "  ".repeat(depth);
            if path.is_dir() {
                println!("{}{}/", indent, relative.display());
                walk_dir(base, &path, depth + 1);
            } else {
                println!("{}{}", indent, relative.display());
            }
        }
    }
}

fn get_manifest_paths(input: &Path) -> Vec<String> {
    let manifest_path = input.join("fxmanifest.lua");
    let content = match fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut paths = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        let has_key = line.contains("_scripts")
            || line.contains("_script")
            || line.contains("files")
            || line.contains("ui_page")
            || line.contains("loadscreen");

        if !has_key || !line.contains('{') {
            i += 1;
            continue;
        }

        let start = line.find('{').unwrap();
        let mut block = String::from(&line[start + 1..]);

        let open = line[start..].matches('{').count();
        let close = line[start..].matches('}').count();
        let mut depth = open - close;

        i += 1;

        while depth > 0 && i < lines.len() {
            let l = lines[i].trim();
            depth += l.matches('{').count();
            depth -= l.matches('}').count();
            block.push(' ');
            block.push_str(l);
            i += 1;
        }

        if let Some(end) = block.rfind('}') {
            block = block[..end].to_string();
        }

        for part in block.split(',') {
            let cleaned = part
                .trim()
                .trim_matches('\'')
                .trim_matches('"')
                .trim();
            if !cleaned.is_empty() && !cleaned.starts_with("--") {
                paths.push(cleaned.to_string());
            }
        }
    }

    paths
}

fn validate_manifest_files(input: &Path) -> Vec<String> {
    let manifest_path = input.join("fxmanifest.lua");
    if !manifest_path.exists() {
        eprintln!("Error: missing fxmanifest.lua");
        std::process::exit(1);
    }

    let manifest_paths = get_manifest_paths(input);
    if manifest_paths.is_empty() {
        eprintln!("Error: no script files declared in fxmanifest.lua");
        std::process::exit(1);
    }

    for p in &manifest_paths {
        let full_path = input.join(p);
        if !full_path.exists() {
            eprintln!("Error: manifest references '{}' but file not found", p);
            std::process::exit(1);
        }
    }

    manifest_paths
}

fn read_file_bytes(path: &Path) -> Vec<u8> {
    let mut buf = Vec::new();
    fs::File::open(path)
        .unwrap()
        .read_to_end(&mut buf)
        .unwrap();
    buf
}

fn pack_zip(input: &Path, output: &Path, paths: &[String]) {
    let file = fs::File::create(output).unwrap_or_else(|e| {
        eprintln!("Error: cannot create output '{}': {}", output.display(), e);
        std::process::exit(1);
    });

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().unix_permissions(0o644);

    let manifest = input.join("fxmanifest.lua");
    if manifest.exists() {
        zip.start_file("fxmanifest.lua", options).unwrap();
        zip.write_all(&read_file_bytes(&manifest)).unwrap();
    }

    for p in paths {
        let full = input.join(p);
        zip.start_file(p, options).unwrap();
        zip.write_all(&read_file_bytes(&full)).unwrap();
        println!("  packed: {}", p);
    }

    zip.finish().unwrap();
}

fn pack_targz(input: &Path, output: &Path, paths: &[String]) {
    let file = fs::File::create(output).unwrap_or_else(|e| {
        eprintln!("Error: cannot create output '{}': {}", output.display(), e);
        std::process::exit(1);
    });

    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(encoder);

    let manifest = input.join("fxmanifest.lua");
    if manifest.exists() {
        let bytes = read_file_bytes(&manifest);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o644);
        header.set_size(bytes.len() as u64);
        tar.append_data(&mut header, "fxmanifest.lua", &bytes[..])
            .unwrap();
    }

    for p in paths {
        let full = input.join(p);
        let bytes = read_file_bytes(&full);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o644);
        header.set_size(bytes.len() as u64);
        tar.append_data(&mut header, p, &bytes[..]).unwrap();
        println!("  packed: {}", p);
    }

    tar.finish().unwrap();
}

fn create_pack(input: &Path, output: &Path, paths: &[String]) {
    let name = output.to_string_lossy();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        pack_targz(input, output, paths);
    } else if name.ends_with(".zip") {
        pack_zip(input, output, paths);
    } else {
        eprintln!("Error: unsupported format '{}' (use .zip, .tar.gz, or .tgz)", name);
        std::process::exit(1);
    }
}

fn pack_resource(input: &Path, output_dir: &Path, name: &str, version: &str) {
    if !input.is_dir() {
        eprintln!("Error: input '{}' is not a directory", input.display());
        std::process::exit(1);
    }

    println!("Packing resource: {}", input.display());
    println!("Name: {}-{}", name, version);

    let filename = format!("{}-{}.zip", name, version);
    let output_path = output_dir.join(&filename);

    display_files(input);

    let paths = validate_manifest_files(input);
    println!("\nCreating pack...");
    create_pack(input, &output_path, &paths);

    println!("\nOutput: {}", output_path.display());
    println!("Pack command executed successfully");
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Pack { input, output, name, version } => {
            let input_path = Path::new(input);
            let output_dir = Path::new(output);
            pack_resource(input_path, output_dir, name, version);
        }
    }
}
