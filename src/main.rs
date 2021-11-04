use std::collections::HashMap;

use anyhow::{Error, Result};
use flexi_logger::{Duplicate, FileSpec, Logger};
use structopt::StructOpt;

use rest::{netbox, netshot};

mod common;
mod rest;

#[derive(Debug, StructOpt, Clone)]
#[structopt(
    name = "netbox2netshot",
    about = "Synchronization tool between netbox and netshot"
)]
struct Opt {
    #[structopt(short, long, help = "Enable debug/verbose mode")]
    debug: bool,

    #[structopt(long, help = "The directory to log to", default_value = "logs", env)]
    log_directory: String,

    #[structopt(long, help = "The Netshot API URL", env)]
    netshot_url: String,

    #[structopt(
        long,
        help = "The TLS certificate to use to authenticate to Netshot (PKCS12 format)",
        env
    )]
    netshot_tls_client_certificate: Option<String>,

    #[structopt(long, help = "The optional password for the netshot PKCS12 file", env)]
    netshot_tls_client_certificate_password: Option<String>,

    #[structopt(long, help = "The Netshot token", env, hide_env_values = true)]
    netshot_token: String,

    #[structopt(long, help = "The domain ID to use when importing a new device", env)]
    netshot_domain_id: u32,

    #[structopt(long, help = "HTTP(s) proxy to use to connect to Netshot", env)]
    netshot_proxy: Option<String>,

    #[structopt(long, help = "The Netbox API URL", env)]
    netbox_url: String,

    #[structopt(
        long,
        help = "The TLS certificate to use to authenticate to Netbox (PKCS12 format)",
        env
    )]
    netbox_tls_client_certificate: Option<String>,

    #[structopt(long, help = "The optional password for the netbox PKCS12 file", env)]
    netbox_tls_client_certificate_password: Option<String>,

    #[structopt(long, help = "The Netbox token", env, hide_env_values = true)]
    netbox_token: Option<String>,

    #[structopt(
        long,
        default_value = "",
        help = "The querystring to use to select the devices from netbox",
        env
    )]
    netbox_devices_filter: String,

    #[structopt(
        long,
        help = "The querystring to use to select the VM from netbox",
        env
    )]
    netbox_vms_filter: Option<String>,

    #[structopt(long, help = "HTTP(s) proxy to use to connect to Netbox", env)]
    netbox_proxy: Option<String>,

    #[structopt(short, long, help = "Check mode, will not push any change to Netshot")]
    check: bool,
}

/// Main application entrypoint
fn main() -> Result<(), Error> {
    let opt: Opt = Opt::from_args();
    let mut logging_level = "info";
    let mut duplicate_level = Duplicate::Info;
    if opt.debug {
        logging_level = "debug";
        duplicate_level = Duplicate::Debug;
    }

    Logger::try_with_str(logging_level)?
        .log_to_file(FileSpec::default().directory(opt.clone().log_directory))
        .duplicate_to_stdout(duplicate_level)
        .start()
        .unwrap();

    log::info!("Logger initialized with level {}", logging_level);
    log::debug!("CLI Parameters : {:#?}", opt);

    let netbox_client = netbox::NetboxClient::new(
        opt.netbox_url,
        opt.netbox_token,
        opt.netbox_proxy,
        opt.netbox_tls_client_certificate,
        opt.netbox_tls_client_certificate_password,
    )?;
    netbox_client.ping()?;

    let netshot_client = netshot::NetshotClient::new(
        opt.netshot_url,
        opt.netshot_token,
        opt.netshot_proxy,
        opt.netshot_tls_client_certificate,
        opt.netshot_tls_client_certificate_password,
    )?;
    netshot_client.ping()?;

    log::info!("Getting devices list from Netshot");
    let netshot_devices = netshot_client.get_devices()?;

    log::debug!("Building netshot devices hashmap");
    let netshot_hashmap: HashMap<_, _> = netshot_devices
        .into_iter()
        .map(|dev| (dev.management_address.ip, dev.name))
        .collect();

    log::info!("Getting devices list from Netbox");
    let mut netbox_devices = netbox_client.get_devices(&opt.netbox_devices_filter)?;

    if opt.netbox_vms_filter.is_some() {
        log::info!("Getting VMS list rom Netbox");
        let mut vms = netbox_client.get_vms(&opt.netbox_vms_filter.unwrap())?;
        log::debug!("Merging VMs and Devices lists");
        netbox_devices.append(&mut vms);
    }

    log::debug!("Building netbox devices hashmap");
    let netbox_hashmap: HashMap<_, _> = netbox_devices
        .into_iter()
        .filter_map(|device| match device.primary_ip4 {
            Some(x) => Some((
                x.address.split("/").next().unwrap().to_owned(),
                device.name.unwrap_or(device.id.to_string()),
            )),
            None => {
                log::warn!(
                    "Device {} is missing its primary IP address, skipping it",
                    device.name.unwrap_or(device.id.to_string())
                );
                None
            }
        })
        .collect();

    log::debug!(
        "Hashmaps: Netbox({}), Netshot({})",
        netbox_hashmap.len(),
        netshot_hashmap.len()
    );

    log::debug!("Comparing HashMaps");
    let mut missing_devices: Vec<String> = Vec::new();
    for (ip, hostname) in netbox_hashmap {
        match netshot_hashmap.get(&ip) {
            Some(x) => log::debug!("{}({}) is present on both", x, ip),
            None => {
                log::debug!("{}({}) missing from Netshot", hostname, ip);
                missing_devices.push(ip);
            }
        }
    }

    log::info!("Found {} devices missing on Netshot", missing_devices.len());

    if !opt.check {
        for device in missing_devices {
            let registration = netshot_client.register_device(&device, opt.netshot_domain_id);
            if let Err(error) = registration {
                log::warn!("Registration failure: {}", error);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use flexi_logger::{AdaptiveFormat, Logger};

    #[ctor::ctor]
    fn enable_logging() {
        Logger::try_with_str("info")
            .unwrap()
            .adaptive_format_for_stderr(AdaptiveFormat::Detailed);
    }
}
