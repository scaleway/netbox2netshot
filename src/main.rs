use std::collections::HashMap;

use anyhow::{Error, Result};
use flexi_logger::{Duplicate, Logger, LogTarget};
use structopt::StructOpt;

use rest::{netbox, netshot};

mod common;
mod rest;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "netbox2netshot",
    about = "Synchronization tool between netbox and netshot"
)]
struct Opt {
    #[structopt(short, long, help = "Enable debug/verbose mode")]
    debug: bool,

    #[structopt(long, help = "The Netshot API URL", env)]
    netshot_url: String,

    #[structopt(long, help = "The Netshot token", env, hide_env_values = true)]
    netshot_token: String,

    #[structopt(long, help = "The domain ID to use when importing a new device", env)]
    netshot_domain_id: u32,

    #[structopt(long, help = "The Netbox API URL", env)]
    netbox_url: String,

    #[structopt(
        long,
        help = "The Netbox token",
        env,
        hide_env_values = true,
        default_value = ""
    )]
    netbox_token: String,

    #[structopt(
        long,
        default_value = "",
        help = "The querystring to use to select the devices from netbox",
        env
    )]
    netbox_devices_filter: String,

    #[structopt(short, long, help = "Check mode, will not push any change to Netshot")]
    check: bool,
}

/// Main application entrypoint
#[tokio::main]
async fn main() -> Result<(), Error> {
    let opt = Opt::from_args();
    let mut logging_level = "info";
    let mut duplicate_level = Duplicate::Info;
    if opt.debug {
        logging_level = "debug";
        duplicate_level = Duplicate::Debug;
    }

    Logger::with_str(logging_level)
        .log_target(LogTarget::File)
        .duplicate_to_stdout(duplicate_level)
        .start()?;

    log::info!("Logger initialized with level {}", logging_level);
    log::debug!("CLI Parameters : {:#?}", opt);

    let netbox_client = netbox::NetboxClient::new(opt.netbox_url, opt.netbox_token)?;
    netbox_client.ping().await?;

    let netshot_client = netshot::NetshotClient::new(opt.netshot_url, opt.netshot_token)?;
    netshot_client.ping().await?;

    log::info!("Getting devices list from Netshot");
    let netshot_devices = netshot_client.get_devices().await?;

    log::debug!("Building netshot devices hashmap");
    let netshot_hashmap: HashMap<_, _> = netshot_devices
        .into_iter()
        .map(|dev| (dev.management_address.ip, dev.name))
        .collect();

    log::info!("Getting devices list from Netbox");
    let netbox_devices = netbox_client
        .get_devices(&opt.netbox_devices_filter)
        .await?;

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
            let registration = netshot_client
                .register_device(&device, opt.netshot_domain_id)
                .await;
            if let Err(error) = registration {
                log::warn!("Registration failure: {}", error);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use flexi_logger::{Duplicate, Logger, LogTarget};

    #[ctor::ctor]
    fn enable_logging() {
        Logger::with_str("debug")
            .log_target(LogTarget::StdOut)
            .start()
            .unwrap();
    }
}
