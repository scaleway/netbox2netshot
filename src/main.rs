mod common;
mod netbox;
mod netshot;

use anyhow::{Error, Result};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::collections::HashMap;
use structopt::StructOpt;

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

    #[structopt(
        long,
        help = "The Netshot token",
        env,
        default_value = "",
        hide_env_values = true
    )]
    netshot_token: String,

    #[structopt(long, help = "The Netbox API URL", env)]
    netbox_url: String,

    #[structopt(long, help = "The Netbox token", env, hide_env_values = true)]
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
    let mut logging_level = LevelFilter::Info;
    if opt.debug {
        logging_level = LevelFilter::Debug;
    }

    SimpleLogger::new().with_level(logging_level).init()?;

    log::info!("Logger initialized with level {}", logging_level);
    log::debug!("CLI Parameters : {:#?}", opt);

    let netbox_client = netbox::NetboxClient::new(opt.netbox_url, opt.netbox_token)?;
    let _netbox_ping = netbox_client.ping().await?;

    let netshot_client = netshot::NetshotClient::new(opt.netshot_url, opt.netshot_token)?;
    let _netshot_ping = netshot_client.ping().await?;

    let netshot_devices = netshot_client.get_devices().await?;

    log::debug!("Building netshot device hashmap");
    let mut netshot_hashmap = HashMap::new();
    for device in netshot_devices {
        netshot_hashmap.insert(device.management_address.ip, device.name);
    }

    let netbox_devices = netbox_client
        .get_devices(&opt.netbox_devices_filter)
        .await?;

    log::debug!("Building netbox device hashmap");
    let mut netbox_hashmap = HashMap::new();
    for device in netbox_devices {
        match device.primary_ip {
            Some(x) => netbox_hashmap.insert(
                String::from(x.address.split("/").next().unwrap()),
                device.name.unwrap_or(device.id.to_string()),
            ),
            None => {
                log::warn!(
                    "Device {} is missing its primary IP address, skipping it",
                    device.name.unwrap_or(device.id.to_string())
                );
                continue;
            }
        };
    }

    log::debug!(
        "Hashmaps: Netbox({}), Netshot({})",
        netbox_hashmap.len(),
        netshot_hashmap.len()
    );


    log::debug!("Comparing HashMaps");
    let mut missing_devices: Vec<String> = Vec::new();
    for device in netbox_hashmap {
        match netshot_hashmap.get(&device.0) {
            Some(x) => log::debug!("{}({}) is present on both", x, device.0),
            None => {
                log::debug!("{}({}) missing from Netshot", device.1, device.0);
                missing_devices.push(device.0);
            },
        }
    }

    log::info!("Found {} devices missing on Netshot", missing_devices.len());

    if !opt.check {
        for device in missing_devices {

        }
    }

    Ok(())
}
