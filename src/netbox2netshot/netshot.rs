use super::common::APP_USER_AGENT;
use anyhow::{anyhow, Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    prefix_length: u8,
    #[serde(rename = "addressUsage")]
    address_usage: String,
    ip: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    id: u32,
    name: String,
    #[serde(rename = "mgmtAddress")]
    management_address: ManagementAddress,
    status: String,
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
}
