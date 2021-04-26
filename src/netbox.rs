use reqwest::header::{HeaderMap, HeaderValue};
use std::error::Error;
use serde::{Deserialize, Serialize};

const APP_USER_AGENT: &str = "netbox2netshot";

#[derive(Debug)]
pub struct NetboxClient {
    url: String,
    token: String,
    client: reqwest::Client,
}

/// Represent the primary_ip field from the DCIM device API call
struct PrimaryIP {
    id: u32,
    family: u8,
    address: String,
}

/// Represent the required information from the DCIM device API call
pub struct Device {
    id: u32,
    name: String,
    primary_ip: PrimaryIP,
}

/// Represent the API response from /api/devim/devices call
#[derive(Debug, Serialize, Deserialize)]
struct NetboxDCIMDeviceList {
    count: u32,
    next: String,
    previous: String,
    results: Vec<Device>,
}

impl NetboxClient {
    /// Create a client without authentication
    pub fn new_anonymous(url: String) -> Result<Self, Box<dyn Error>> {
        NetboxClient::new(url, String::from(""))
    }

    /// Create a client with the given authentication token
    pub fn new(url: String, token: String) -> Result<Self, Box<dyn Error>> {
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
    pub async fn ping(&self) -> Result<bool, Box<dyn Error>> {
        let url = format!("{}/api/dcim/devices/?name=netbox2netshot-ping", self.url);
        log::debug!("Pinging {}", url);
        let response = self.client.get(url).send().await?;
        log::debug!("Ping response: {}", response.status());
        Ok(response.status() == 200)
    }

    pub async fn get_devices_page(&self, query_string: String, limit: u8, offset: u8) -> Result<Vec<Device>, Box<dyn Error>> {
        let url = format!("{}/api/dcim/devices/?limit={}&offset={}&{}", self.url, limit, offset, query_string);
        let page : NetboxDCIMDeviceList = self.client.get(url).send().await?.json().await?;
        Ok(response)

    }

    /// Get the devices using the given filter
    pub async fn get_devices(&self, query_string: String) -> Result<Vec<Device>, Box<dyn Error>> {
        let mut devices : Vec<Device> = Vec::new();
        Ok(devices)
    }
}
