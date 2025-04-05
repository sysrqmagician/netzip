use std::time::Duration;

use bittenhumans::{ByteSizeFormatter, consts::System};
use clap::{Parser, Subcommand};
use comfy_table::{Cell, ContentArrangement, Table, presets::ASCII_FULL_CONDENSED};
use indicatif::ProgressBar;
use netzip::*;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(alias = "l")]
    List { url: String },
    #[command(alias = "x")]
    Extract { url: String, files: Vec<String> },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let http_client = reqwest::Client::new();
    let pb = ProgressBar::new_spinner().with_message("Requesting data...");
    pb.enable_steady_tick(Duration::from_millis(100));

    match args.command {
        Commands::Extract { url, files } => {
            match extract_files_from_zip_url(&url, files, http_client).await {
                Err(e) => {
                    pb.finish();
                    eprintln!("{e}");
                }
                Ok(files) => {
                    let mut file_count = 0;
                    for file in files {
                        pb.set_message(format!("Writing to disk: {}", file.0.file_name));
                        if let Err(e) = std::fs::write(
                            &file
                                .0
                                .file_name
                                .split("/")
                                .last()
                                .unwrap_or(&file.0.file_name),
                            file.1,
                        ) {
                            eprintln!("Failed writing {} to disk: {e}", file.0.file_name);
                        } else {
                            file_count += 1;
                        }
                    }
                    pb.finish_with_message(format!("Downloaded {file_count} files."));
                }
            }
        }
        Commands::List { url } => match extract_listing_from_zip_url(&url, http_client).await {
            Err(e) => {
                pb.finish();
                eprintln!("{e}");
            }
            Ok(mut records) => {
                pb.set_message("Processing...");

                let mut table = Table::new();
                table
                    .load_preset(ASCII_FULL_CONDENSED)
                    .set_content_arrangement(ContentArrangement::Dynamic)
                    .set_header(vec![
                        Cell::new("Path").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Compressed Size").add_attribute(comfy_table::Attribute::Bold),
                        Cell::new("Uncompressed Size").add_attribute(comfy_table::Attribute::Bold),
                    ]);

                records.sort_by(|x, y| x.file_name.cmp(&y.file_name));
                for record in records {
                    table.add_row(vec![
                        record.file_name,
                        ByteSizeFormatter::fit(record.compressed_size as u64, System::Binary)
                            .format(record.compressed_size as u64),
                        ByteSizeFormatter::fit(record.uncompressed_size as u64, System::Binary)
                            .format(record.uncompressed_size as u64),
                    ]);
                }

                pb.finish_and_clear();
                println!("{table}");
            }
        },
    }
}
