use super::common::APP_USER_AGENT;
use anyhow::{Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde;
use serde::{Deserialize, Serialize};

const PATH_DEVICES: &str = "/api/devices";

#[derive(Debug)]
pub struct NetshotClient {
    pub url: String,
    pub token: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ManagementAddress {
    #[serde(rename = "prefixLength")]
    pub prefix_length: u8,
    #[serde(rename = "addressUsage")]
    pub address_usage: String,
    pub ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: u32,
    pub name: String,
    #[serde(rename = "mgmtAddress")]
    pub management_address: ManagementAddress,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewDevicePayload {
    #[serde(rename = "autoDiscover")]
    auto_discover: bool,

    #[serde(rename = "ipAddress")]
    ip_address: String,

    #[serde(rename = "domainId")]
    domain_id: u32,
}

impl NetshotClient {
    /// Create a client with the given authentication token
    pub fn new(url: String, token: String) -> Result<Self, Error> {
        log::debug!("Creating new Netshot client to {}", url);
        let mut http_headers = HeaderMap::new();
        let header_value = HeaderValue::from_str(token.as_str())?;
        http_headers.insert("X-Netshot-API-Token", header_value);
        http_headers.insert("Accept", HeaderValue::from_str("application/json")?);
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

    /// To be implemented server side, always return true for now
    pub async fn ping(&self) -> Result<bool, Error> {
        log::warn!("Not health check implemented on Netshot, ping will always succeed");
        Ok(true)
    }

    /// Get devices registered in Netshot
    pub async fn get_devices(&self) -> Result<Vec<Device>, Error> {
        let url = format!("{}{}", self.url, PATH_DEVICES);
        let devices: Vec<Device> = self.client.get(url).send().await?.json().await?;

        log::debug!("Got {} devices from Netshot", devices.len());

        Ok(devices)
    }

    /// Register a given IP into Netshot and return the corresponding device
    pub async fn register_device(&self, ip_address: String, domain_id: u32) -> Result<Device, Error> {

        log::info!("Registering new device with IP {}", ip_address);

        let newDevice = NewDevicePayload{ auto_discover: true, ip_address, domain_id };
        let response = self.client.post(PATH_DEVICES).json(&newDevice).send().await?;
        let device: Device = response.json().await?;

        // TODO: Finish

        Ok(device)
    }
}
