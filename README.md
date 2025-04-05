# NetZip

NetZip is a Rust library and CLI tool that allows you to work with remote ZIP files over HTTP without downloading the entire archive. It uses HTTP range requests to efficiently fetch only the parts of the ZIP file that are needed.

## Features

- 🚀 **Network Efficient** - Download only the parts of the ZIP file you need
- 📋 **List Files** - View the contents of a remote ZIP file
- 📦 **Extract Files** - Download specific files from a remote ZIP
- 🧩 **Library & CLI** - Use as a library in your Rust projects or as a command-line tool

## Installation

### From Cargo

```bash
cargo install netzip_cli
```

### From Source

```bash
git clone https://github.com/sysrqmagician/netzip.git
cd netzip
cargo build --release
```

The binary will be available at `./target/release/netzip_cli`.

## CLI Usage

### List Files in a Remote ZIP

```bash
netzip list https://example.com/archive.zip

# or with the shorter alias
netzip l https://example.com/archive.zip
```

This will display a table with file paths, compressed sizes, and uncompressed sizes.

### Extract Files from a Remote ZIP

```bash
# Extract specific files
netzip extract https://example.com/archive.zip file1.txt file2.txt

# or with the shorter alias
netzip x https://example.com/archive.zip file1.txt file2.txt
```

## Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
netzip = "0.1.0"
reqwest = "0.12.15"
tokio = { version = "1.44.1", features = ["full"] }
```

### Example: List Files in a ZIP

```rust
use netzip::RemoteZip;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com/archive.zip";
    let zip = RemoteZip::get(url).await?;

    for record in zip.records() {
        println!("{} - {} bytes", record.file_name, record.uncompressed_size);
    }

    Ok(())
}
```

### Example: Extract Specific Files

```rust
use netzip::RemoteZip;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://example.com/archive.zip";
    let zip = RemoteZip::get(url).await?;

    let files_to_extract = vec!["file1.txt".to_string(), "file2.txt".to_string()];
    let extracted_files = zip.download_files(files_to_extract).await?;

    for (file, content) in extracted_files {
        fs::write(&file.file_name, content)?;
        println!("Extracted: {}", file.file_name);
    }

    Ok(())
}
```

## How It Works

NetZip uses a three-step process to efficiently access files in a remote ZIP archive:

1. **Fetch End of Central Directory** - First, it downloads just the end of the ZIP file to locate the Central Directory.
2. **Download Central Directory** - It then downloads only the Central Directory, which contains metadata about all files in the archive.
3. **Extract Specific Files** - Finally, it downloads only the parts of the archive that contain the requested files.

This approach minimizes bandwidth usage, making it ideal for working with large ZIP files when you only need specific contents.

## Supported Compression Methods

- Stored (uncompressed)
- Deflate
- Deflate64

## Project Structure

- **netzip_parser**: Low-level ZIP format parser
- **netzip**: Main library for HTTP-based ZIP access
- **netzip_cli**: Command-line interface

## License

LGPLv3
