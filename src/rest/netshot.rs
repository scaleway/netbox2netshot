use crate::common::APP_USER_AGENT;
use crate::rest::helpers::build_identity_from_file;
use anyhow::{anyhow, Error, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Proxy;
use serde;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const PATH_DEVICES: &str = "/api/devices";
const PATH_DEVICES_SEARCH: &str = "/api/devices/search";

#[derive(Debug)]
pub struct NetshotClient {
    pub url: String,
    pub token: String,
    pub client: reqwest::blocking::Client,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct NewDeviceCreatedPayload {
    #[serde(rename = "id")]
    pub task_id: u32,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateDevicePayload {
    enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceUpdatedPayload {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceSearchQueryPayload {
    query: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceSearchResultPayload {
    pub query: String,
    pub devices: Vec<Device>,
}

impl NetshotClient {
    /// Create a client with the given authentication token
    pub fn new(
        url: String,
        token: String,
        proxy: Option<String>,
        tls_client_certificate: Option<String>,
        tls_client_certificate_password: Option<String>,
    ) -> Result<Self, Error> {
        log::debug!("Creating new Netshot client to {}", url);
        let mut http_headers = HeaderMap::new();
        let header_value = HeaderValue::from_str(token.as_str())?;
        http_headers.insert("X-Netshot-API-Token", header_value);
        http_headers.insert("Accept", HeaderValue::from_str("application/json")?);
        let mut http_client = reqwest::blocking::Client::builder()
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::from_secs(5))
            .default_headers(http_headers);

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
            token,
            client: http_client.build()?,
        })
    }

    /// To be implemented server side, always return true for now
    pub fn ping(&self) -> Result<bool, Error> {
        log::warn!("Not health check implemented on Netshot, ping will always succeed");
        Ok(true)
    }

    /// Get devices registered in Netshot
    pub fn get_devices(&self) -> Result<Vec<Device>, Error> {
        let url = format!("{}{}", self.url, PATH_DEVICES);
        let devices: Vec<Device> = self.client.get(url).send()?.json()?;

        log::debug!("Got {} devices from Netshot", devices.len());

        Ok(devices)
    }

    /// Register a given IP into Netshot and return the corresponding device
    pub fn register_device(
        &self,
        ip_address: String,
        domain_id: u32,
    ) -> Result<NewDeviceCreatedPayload, Error> {
        log::info!("Registering new device with IP {}", ip_address);

        let new_device = NewDevicePayload {
            auto_discover: true,
            ip_address: ip_address.clone(),
            domain_id,
        };

        let url = format!("{}{}", self.url, PATH_DEVICES);
        let response = self.client.post(url).json(&new_device).send()?;

        if !response.status().is_success() {
            log::warn!(
                "Failed to register new device {}, got status {}",
                ip_address,
                response.status().to_string()
            );
            return Err(anyhow!("Failed to register new device {}", ip_address));
        }

        let device_registration: NewDeviceCreatedPayload = response.json()?;
        log::debug!(
            "Device registration for device {} requested with task ID {}",
            ip_address,
            device_registration.task_id
        );

        Ok(device_registration)
    }

    /// Search for a device
    pub fn search_device(&self, query_string: String) -> Result<DeviceSearchResultPayload, Error> {
        let url = format!("{}{}", self.url, PATH_DEVICES_SEARCH);

        let query = DeviceSearchQueryPayload {
            query: query_string.clone(),
        };

        let response = self.client.post(url).json(&query).send()?;

        if !response.status().is_success() {
            log::warn!(
                "Failed to search for device with query `{}`: {}",
                query_string.clone(),
                response.status().to_string()
            );
            return Err(anyhow!(
                "Failed to search for device with query: {}",
                query_string
            ));
        }

        let search_result: DeviceSearchResultPayload = response.json()?;
        log::debug!(
            "Found {} devices with the given search",
            search_result.devices.len(),
        );

        Ok(search_result)
    }

    /// Set the given device to a given state (enabled/disabled)
    fn set_device_enabled(
        &self,
        ip_address: String,
        enabled: bool,
    ) -> Result<Option<DeviceUpdatedPayload>, Error> {
        log::info!(
            "Setting device with IP {} to enabled={}",
            ip_address,
            enabled
        );

        let state = UpdateDevicePayload { enabled: enabled };

        // Search for the device ID
        let response = self.search_device(format!("[IP] IS {}", ip_address))?;
        let device = response.devices.first().unwrap();

        if !enabled && device.status == "DISABLED" {
            log::warn!(
                "Device {}({}) is already disabled, skipping",
                device.name,
                ip_address
            );
            return Ok(Option::None);
        } else if enabled && device.status != "DISABLED" {
            log::warn!(
                "Device {}({}) is already enabled, skipping",
                device.name,
                ip_address
            );
            return Ok(Option::None);
        }

        let url = format!("{}{}/{}", self.url, PATH_DEVICES, device.id);
        let response = self.client.put(url).json(&state).send()?;

        if !response.status().is_success() {
            log::warn!(
                "Failed to update state for device {}, got status {}",
                ip_address,
                response.status().to_string()
            );
            return Err(anyhow!(
                "Failed to update state for device {}, got status {}",
                ip_address,
                response.status().to_string()
            ));
        }

        let device_update: DeviceUpdatedPayload = response.json()?;
        log::debug!("Device state of {} set to enabled={}", ip_address, enabled);

        Ok(Option::Some(device_update))
    }

    /// Disable a given device
    pub fn disable_device(
        &self,
        ip_address: String,
    ) -> Result<Option<DeviceUpdatedPayload>, Error> {
        self.set_device_enabled(ip_address, false)
    }

    /// Enable a given device
    pub fn enable_device(&self, ip_address: String) -> Result<Option<DeviceUpdatedPayload>, Error> {
        self.set_device_enabled(ip_address, true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    #[test]
    fn authenticated_initialization() {
        let url = mockito::server_url();
        let token = String::from("hello");
        let client = NetshotClient::new(url.clone(), token.clone(), None, None, None).unwrap();
        assert_eq!(client.token, token);
        assert_eq!(client.url, url);
    }

    #[test]
    fn single_good_device() {
        let url = mockito::server_url();

        let _mock = mockito::mock("GET", PATH_DEVICES)
            .match_query(mockito::Matcher::Any)
            .with_body_from_file("tests/data/netshot/single_good_device.json")
            .create();

        let client = NetshotClient::new(url.clone(), String::new(), None, None, None).unwrap();
        let devices = client.get_devices().unwrap();

        assert_eq!(devices.len(), 1);

        let device = devices.first().unwrap();

        assert_eq!(device.name, "test-device");
        assert_eq!(device.id, 1 as u32);
        assert_eq!(device.management_address.ip, "1.2.3.4");
    }

    #[test]
    fn good_device_registration() {
        let url = mockito::server_url();

        let _mock = mockito::mock("POST", PATH_DEVICES)
            .match_query(mockito::Matcher::Any)
            .match_body(r#"{"autoDiscover":true,"ipAddress":"1.2.3.4","domainId":2}"#)
            .with_body_from_file("tests/data/netshot/good_device_registration.json")
            .create();

        let client = NetshotClient::new(url.clone(), String::new(), None, None, None).unwrap();
        let registration = client.register_device(String::from("1.2.3.4"), 2).unwrap();

        assert_eq!(registration.task_id, 504);
        assert_eq!(registration.status, "SCHEDULED");
    }

    #[test]
    fn search_devices() {
        let url = mockito::server_url();

        let _mock = mockito::mock("POST", PATH_DEVICES_SEARCH)
            .match_query(mockito::Matcher::Any)
            .match_body(r#"{"query":"[IP] IS 1.2.3.4"}"#)
            .with_body_from_file("tests/data/netshot/search.json")
            .create();

        let client = NetshotClient::new(url.clone(), String::new(), None, None, None).unwrap();
        let result = client
            .search_device(String::from("[IP] IS 1.2.3.4"))
            .unwrap();

        assert_eq!(result.devices.len(), 2);
        assert_eq!(result.query, "[IP] IS 1.2.3.4");
    }

    #[test]
    fn disable_device() {
        let url = mockito::server_url();

        let _mock = mockito::mock("PUT", format!("{}/{}", PATH_DEVICES, 2318).as_str())
            .match_query(mockito::Matcher::Any)
            .match_body(r#"{"enabled":false}"#)
            .with_body_from_file("tests/data/netshot/disable_device.json")
            .create();

        let _mock2 = mockito::mock("POST", PATH_DEVICES_SEARCH)
            .match_query(mockito::Matcher::Any)
            .match_body(r#"{"query":"[IP] IS 1.2.3.4"}"#)
            .with_body_from_file("tests/data/netshot/search.json")
            .create();

        let client = NetshotClient::new(url.clone(), String::new(), None, None, None).unwrap();
        let registration = client.disable_device(String::from("1.2.3.4")).unwrap();

        assert_eq!(registration.unwrap().status, "DISABLED");
    }
}
