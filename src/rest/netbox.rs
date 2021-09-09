use crate::common::APP_USER_AGENT;
use crate::rest::helpers::build_identity_from_file;
use anyhow::{anyhow, Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Proxy;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_LIMIT: u32 = 100;
const PATH_PING: &str = "/api/dcim/devices/?name=netbox2netshot-ping";
const PATH_DCIM_DEVICES: &str = "/api/dcim/devices/";
const PATH_VIRT_VM: &str = "/api/virtualization/virtual-machines/";

/// The Netbox client
#[derive(Debug)]
pub struct NetboxClient {
    pub url: String,
    pub token: String,
    pub client: reqwest::blocking::Client,
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
    pub primary_ip4: Option<PrimaryIP>,
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
    let offset_string = url.query_pairs().find(|(key, _)| key == "offset");
    match offset_string {
        Some((_, x)) => Ok(x.parse()?),
        None => Err(anyhow!("No offset found in url")),
    }
}

impl Device {
    /// Is this a valid device for import
    pub fn is_valid(&self) -> bool {
        self.primary_ip4.is_some() && self.name.is_some()
    }
}

impl NetboxClient {
    /// Create a client without authentication
    pub fn new_anonymous(url: String, proxy: Option<String>) -> Result<Self, Error> {
        NetboxClient::new(url, None, proxy, None, None)
    }

    /// Create a client with the given authentication token
    pub fn new(
        url: String,
        token: Option<String>,
        proxy: Option<String>,
        tls_client_certificate: Option<String>,
        tls_client_certificate_password: Option<String>,
    ) -> Result<Self, Error> {
        log::debug!("Creating new Netbox client to {}", url);
        let mut http_client = reqwest::blocking::Client::builder()
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::from_secs(5));

        http_client = match token {
            Some(ref t) => {
                let mut http_headers = HeaderMap::new();
                let header_value = HeaderValue::from_str(format!("Token {}", t).as_str())?;
                http_headers.insert("Authorization", header_value);
                http_client.default_headers(http_headers)
            }
            None => http_client,
        };

        http_client = match proxy {
            Some(p) => http_client.proxy(Proxy::all(p)?),
            None => http_client,
        };

        http_client = match tls_client_certificate {
            Some(c) => http_client.identity(build_identity_from_file(
                c,
                tls_client_certificate_password,
            )?),
            None => http_client,
        };

        Ok(Self {
            url,
            token: token.unwrap_or("".to_string()),
            client: http_client.build()?,
        })
    }

    /// Ping the service to make sure it is reachable and pass the authentication (if there is any)
    pub fn ping(&self) -> Result<bool, Error> {
        let url = format!("{}{}", self.url, PATH_PING);
        log::debug!("Pinging {}", url);
        let response = self.client.get(url).send()?;
        log::debug!("Ping response: {}", response.status());
        Ok(response.status().is_success())
    }

    /// Get a single device page
    pub fn get_devices_page(
        &self,
        path: &str,
        query_string: &String,
        limit: u32,
        offset: u32,
    ) -> Result<NetboxDCIMDeviceList, Error> {
        let url = format!(
            "{}{}?limit={}&offset={}&{}",
            self.url, path, limit, offset, query_string
        );
        let page: NetboxDCIMDeviceList = self.client.get(url).send()?.json()?;
        Ok(page)
    }

    /// Get the devices using the given filter
    pub fn get_devices(&self, query_string: &String) -> Result<Vec<Device>, Error> {
        let mut devices: Vec<Device> = Vec::new();
        let mut offset = 0;

        loop {
            let mut response =
                self.get_devices_page(PATH_DCIM_DEVICES, &query_string, API_LIMIT, offset)?;

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

    /// Get the VMs as device using the given filter
    pub fn get_vms(&self, query_string: &String) -> Result<Vec<Device>, Error> {
        let mut devices: Vec<Device> = Vec::new();
        let mut offset = 0;

        loop {
            let mut response =
                self.get_devices_page(PATH_VIRT_VM, &query_string, API_LIMIT, offset)?;

            devices.append(&mut response.results);

            let pages_count = response.count / API_LIMIT;
            log::debug!(
                "Got {} VM devices on the {} matches (page {}/{})",
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

        log::info!("Fetched {} VM devices from Netbox", devices.len());
        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    #[test]
    fn anonymous_initialization() {
        let url = mockito::server_url();
        let client = NetboxClient::new_anonymous(url.clone(), None).unwrap();
        assert_eq!(client.token, "");
        assert_eq!(client.url, url);
    }

    #[test]
    fn authenticated_initialization() {
        let url = mockito::server_url();
        let token = String::from("hello");
        let client = NetboxClient::new(url.clone(), Some(token.clone()), None, None, None).unwrap();
        assert_eq!(client.token, token);
        assert_eq!(client.url, url);
    }

    #[test]
    fn failed_ping() {
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", mockito::Matcher::Any)
            .with_status(403)
            .create();

        let client = NetboxClient::new_anonymous(url.clone(), None).unwrap();
        let ping = client.ping().unwrap();
        assert_eq!(ping, false);
    }

    #[test]
    fn successful_ping() {
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_PING)
            .with_body_from_file("tests/data/netbox/ping.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone(), None).unwrap();
        let ping = client.ping().unwrap();
        assert_eq!(ping, true);
    }

    #[test]
    fn single_good_device() {
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DCIM_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/netbox/single_good_device.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone(), None).unwrap();
        let devices = client.get_devices(&String::from("")).unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.name.as_ref().unwrap(), "test-device");
        assert_eq!(device.id, 1 as u32);
        assert_eq!(device.primary_ip4.as_ref().unwrap().address, "1.2.3.4/32");
        assert_eq!(device.is_valid(), true);
    }

    #[test]
    fn single_device_without_primary_ip() {
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DCIM_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/netbox/single_device_without_primary_ip.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone(), None).unwrap();
        let devices = client.get_devices(&String::from("")).unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.is_valid(), false);
    }

    #[test]
    fn single_device_without_name() {
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DCIM_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/netbox/single_device_without_name.json")
            .create();

        let client = NetboxClient::new_anonymous(url.clone(), None).unwrap();
        let devices = client.get_devices(&String::from("")).unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.is_valid(), false);
    }
}
