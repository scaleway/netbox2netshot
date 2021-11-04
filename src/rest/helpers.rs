use anyhow::{Error};
use reqwest::Identity;
use std::fs::File;
use std::io::Read;

/// Create an identity from a private key and certificate registered in a PKCS12 file (with or without password)
pub fn build_identity_from_file(
    filename: String,
    password: Option<String>,
) -> Result<Identity, Error> {
    let mut buf = Vec::new();
    File::open(filename.as_str())?.read_to_end(&mut buf)?;

    log::info!("Building identity from {} PFX/P12 file", filename);
    let identity = match password {
        Some(p) => Identity::from_pkcs12_der(&buf, p.as_str())?,
        None => Identity::from_pkcs12_der(&buf, "")?,
    };

    Ok(identity)
}
