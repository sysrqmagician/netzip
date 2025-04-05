use std::io::Read;

use bytes::Bytes;
use flate2::bufread::DeflateDecoder;
use netzip_parser::{CentralDirectoryEnd, CentralDirectoryRecord, LocalFile, ZipError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error encountered while sending network request to '{0}': {1}")]
    NetworkError(String, reqwest::Error),
    #[error("Error encountered whiler parsing Zip from request to '{0}': {1}")]
    ParserError(String, ZipError),
    #[error("Error encountered while decompressing file from request to '{0}': {1}")]
    DecompressionError(String, String),
    #[error("Unable to decompress file with compression type {0}")]
    UnsupportCompression(u16),
}

pub async fn extract_listing_from_zip_url(
    url: &str,
    client: reqwest::Client,
) -> Result<Vec<CentralDirectoryRecord>, Error> {
    let min_cde_bytes = ranged_request(
        url,
        &format!("bytes=-{}", netzip_parser::EOCD_MIN_SIZE),
        client.clone(),
    )
    .await?;

    let cde = if let Ok(min_out) = CentralDirectoryEnd::parse(&min_cde_bytes) {
        min_out
    } else {
        // There might be a comment, retry with an offset and search for the EOCD
        let cde_haystack = ranged_request(
            url,
            &format!("bytes=-{}", netzip_parser::EOCD_MIN_SIZE + 1024),
            client.clone(),
        )
        .await?;

        CentralDirectoryEnd::find_and_parse(&cde_haystack)
            .map_err(|e| Error::ParserError(url.into(), e))?
    };

    let cd_bytes = ranged_request(
        url,
        &format!(
            "bytes={}-{}",
            cde.central_directory_offset,
            cde.central_directory_offset + cde.directory_size
        ),
        client,
    )
    .await?;

    let cd_records = CentralDirectoryRecord::parse_many(&cd_bytes)
        .map_err(|e| Error::ParserError(url.into(), e))?;

    Ok(cd_records)
}

pub async fn extract_files_from_zip_url(
    url: &str,
    file_paths: Vec<String>,
    client: reqwest::Client,
) -> Result<Vec<(LocalFile, Vec<u8>)>, Error> {
    let cd_records = extract_listing_from_zip_url(url.into(), client.clone()).await?;
    let needed_cd_records: Vec<&CentralDirectoryRecord> = cd_records
        .iter()
        .filter(|x| file_paths.contains(&x.file_name))
        .collect();

    let mut out = Vec::new();

    for cd_record in needed_cd_records {
        let lfh_end_offset = cd_record.file_header_offset
            + netzip_parser::LFH_MIN_SIZE as u32
            + cd_record.extra_field_length as u32
            + cd_record.file_name_length as u32
            + cd_record.file_comment_length as u32;
        let lfh_bytes = ranged_request(
            url,
            &format!("bytes={}-{}", cd_record.file_header_offset, lfh_end_offset),
            client.clone(),
        )
        .await?;

        let lfh = LocalFile::parse(&lfh_bytes).map_err(|e| Error::ParserError(url.into(), e))?;

        match lfh.compression_method {
            netzip_parser::CompressionMethod::Deflate
            | netzip_parser::CompressionMethod::Deflate64 => {
                let compressed_data: &[u8] = &ranged_request(
                    url,
                    &format!(
                        "bytes={}-{}",
                        lfh_end_offset,
                        lfh_end_offset + lfh.compressed_size
                    ),
                    client.clone(),
                )
                .await?;

                let mut decoder = DeflateDecoder::new(compressed_data);
                let mut decoded = Vec::with_capacity(lfh.uncompressed_size as usize);
                decoder
                    .read_to_end(&mut decoded)
                    .map_err(|e| Error::DecompressionError(url.into(), e.to_string()))?;

                out.push((lfh, decoded));
            }
            netzip_parser::CompressionMethod::Stored => {
                let data = ranged_request(
                    url,
                    &format!(
                        "bytes={}-{}",
                        lfh_end_offset,
                        lfh_end_offset + lfh.uncompressed_size
                    ),
                    client.clone(),
                )
                .await?;

                out.push((lfh, data.into()));
            }
            netzip_parser::CompressionMethod::Unsupported(unsupported_id) => {
                return Err(Error::UnsupportCompression(unsupported_id));
            }
        }
    }

    return Ok(out);
}

async fn ranged_request(
    url: &str,
    range_string: &str,
    client: reqwest::Client,
) -> Result<Bytes, Error> {
    client
        .get(url)
        .header("Range", range_string)
        .send()
        .await
        .map_err(|e| Error::NetworkError(url.into(), e))?
        .bytes()
        .await
        .map_err(|e| Error::NetworkError(url.into(), e))
}
