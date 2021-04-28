use super::common::APP_USER_AGENT;
use anyhow::{anyhow, Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const API_LIMIT: u32 = 100;
const PATH_PING: &str = "/api/dcim/devices/?name=netbox2netshot-ping";
const PATH_DCIM_DEVICES: &str = "/api/dcim/devices/";

#[derive(Debug)]
pub struct NetboxClient {
    pub url: String,
    pub token: String,
    pub client: reqwest::Client,
}

/// Represent the primary_ip field from the DCIM device API call
#[derive(Debug, Serialize, Deserialize)]
pub struct PrimaryIP {
    pub id: u32,
    pub family: u8,
    pub address: String,
}

/// Represent the required information from the DCIM device API call
#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: u32,
    pub name: Option<String>,
    pub primary_ip: Option<PrimaryIP>,
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

impl Device {
    pub fn is_valid(&self) -> bool {
        self.primary_ip.is_some() && self.name.is_some()
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
            .default_headers(http_headers)
            .build()?;
        Ok(Self {
            url,
            token,
            client: http_client,
        })
    }

    /// Ping the service to make sure it is reachable and pass the authentication (if there is any)
    pub async fn ping(&self) -> Result<bool, Error> {
        let url = format!("{}{}", self.url, PATH_PING);
        log::debug!("Pinging {}", url);
        let response = self.client.get(url).send().await?;
        log::debug!("Ping response: {}", response.status());
        Ok(response.status().is_success())
    }

    /// Get a single device page
    pub async fn get_devices_page(
        &self,
        query_string: &String,
        limit: u32,
        offset: u32,
    ) -> Result<NetboxDCIMDeviceList, Error> {
        let url = format!(
            "{}{}?limit={}&offset={}&{}",
            self.url, PATH_DCIM_DEVICES, limit, offset, query_string
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    fn enable_logging() {
        let _ = simple_logger::SimpleLogger::new()
            .with_level(log::LevelFilter::Debug)
            .init();
    }

    #[test]
    fn anonymous_initialization() {
        enable_logging();
        let url = mockito::server_url();
        let client = NetboxClient::new_anonymous(url.clone()).unwrap();
        assert_eq!(client.token, "");
        assert_eq!(client.url, url);
    }

    #[test]
    fn authenticated_initialization() {
        enable_logging();
        let url = mockito::server_url();
        let token = String::from("hello");
        let client = NetboxClient::new(url.clone(), token.clone()).unwrap();
        assert_eq!(client.token, token);
        assert_eq!(client.url, url);
    }

    #[tokio::test]
    async fn failed_ping() {
        enable_logging();
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", mockito::Matcher::Any)
            .with_status(403)
            .create();

        let client = NetboxClient::new_anonymous(url.clone()).unwrap();
        let ping = client.ping().await.unwrap();
        assert_eq!(ping, false);
    }

    #[tokio::test]
    async fn successful_ping() {
        enable_logging();
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_PING)
            .with_body_from_file("tests/data/ping.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone()).unwrap();
        let ping = client.ping().await.unwrap();
        assert_eq!(ping, true);
    }

    #[tokio::test]
    async fn single_good_device() {
        enable_logging();
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DCIM_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/single_good_device.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone()).unwrap();
        let devices = client.get_devices(&String::from("")).await.unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.name.as_ref().unwrap(), "test-device");
        assert_eq!(device.id, 1 as u32);
        assert_eq!(device.primary_ip.as_ref().unwrap().address, "1.2.3.4/32");
        assert_eq!(device.is_valid(), true);
    }

    #[tokio::test]
    async fn single_device_without_primary_ip() {
        enable_logging();
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DCIM_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/single_device_without_primary_ip.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone()).unwrap();
        let devices = client.get_devices(&String::from("")).await.unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.is_valid(), false);
    }

    #[tokio::test]
    async fn single_device_without_name() {
        enable_logging();
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DCIM_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/single_device_without_name.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone()).unwrap();
        let devices = client.get_devices(&String::from("")).await.unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.is_valid(), false);
    }
}
