use anyhow::{anyhow, Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const APP_USER_AGENT: &str = "netbox2netshot";
const API_LIMIT: u32 = 100;

#[derive(Debug)]
pub struct NetboxClient {
    url: String,
    token: String,
    client: reqwest::Client,
}

/// Represent the primary_ip field from the DCIM device API call
#[derive(Debug, Serialize, Deserialize)]
pub struct PrimaryIP {
    id: u32,
    family: u8,
    address: String,
}

/// Represent the required information from the DCIM device API call
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    id: u32,
    name: Option<String>,
    primary_ip: Option<PrimaryIP>,
}

/// Represent the API response from /api/devim/devices call
#[derive(Debug, Serialize, Deserialize)]
pub struct NetboxDCIMDeviceList {
    count: u32,
    next: Option<String>,
    previous: Option<String>,
    results: Vec<Device>,
}

/// Extract the offset from the URL returned from the API
fn extract_offset(url_string: &String) -> Result<u32, Error> {
    let url = reqwest::Url::parse(url_string)?;
    let args: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let offset_string = args.get("offset");
    match offset_string {
        Some(x) => Ok(x.parse()?),
        None => Err(anyhow!("No offset found in url")),
    }
}

impl NetboxClient {
    /// Create a client without authentication
    pub fn new_anonymous(url: String) -> Result<Self, Error> {
        NetboxClient::new(url, String::from(""))
    }

    /// Create a client with the given authentication token
    pub fn new(url: String, token: String) -> Result<Self, Error> {
        log::debug!("Creating new Netbox client to {}", url);
        let mut http_headers = HeaderMap::new();
        if token != "" {
            let header_value = HeaderValue::from_str(format!("Token {}", token).as_str())?;
            http_headers.insert("Authorization", header_value);
        }
        let http_client = reqwest::Client::builder()
            .user_agent(APP_USER_AGENT)
            .build()?;
        Ok(Self {
            url,
            token,
            client: http_client,
        })
    }

    /// Ping the service to make sure it is reachable and pass the authentication (if there is any)
    pub async fn ping(&self) -> Result<bool, Error> {
        let url = format!("{}/api/dcim/devices/?name=netbox2netshot-ping", self.url);
        log::debug!("Pinging {}", url);
        let response = self.client.get(url).send().await?;
        log::debug!("Ping response: {}", response.status());
        Ok(response.status() == 200)
    }

    /// Get a single device page
    pub async fn get_devices_page(
        &self,
        query_string: &String,
        limit: u32,
        offset: u32,
    ) -> Result<NetboxDCIMDeviceList, Error> {
        let url = format!(
            "{}/api/dcim/devices/?limit={}&offset={}&{}",
            self.url, limit, offset, query_string
        );
        let page: NetboxDCIMDeviceList = self.client.get(url).send().await?.json().await?;
        Ok(page)
    }

    /// Get the devices using the given filter
    pub async fn get_devices(&self, query_string: &String) -> Result<Vec<Device>, Error> {
        let mut devices: Vec<Device> = Vec::new();
        let mut offset = 0;

        loop {
            let mut response = self
                .get_devices_page(&query_string, API_LIMIT, offset)
                .await?;

            devices.append(&mut response.results);

            let pages_count = response.count / API_LIMIT;
            log::debug!(
                "Got {} devices on the {} matches (page {}/{})",
                devices.len(),
                response.count,
                (offset / API_LIMIT),
                pages_count
            );

            match response.next {
                Some(x) => {
                    offset = extract_offset(&x)?;
                }
                None => break,
            }
        }

        log::info!("Fetched {} devices from Netbox", devices.len());
        Ok(devices)
    }
}
