mod netbox;

use anyhow::{Error, Result};
use log::LevelFilter;
use simple_logger::SimpleLogger;
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
    let netbox_ping = netbox_client.ping().await?;

    let netbox_devices = netbox_client
        .get_devices(&opt.netbox_devices_filter)
        .await?;

    Ok(())
}
