mod netbox;

extern crate serde;

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

    #[structopt(long, default_value = "", help = "The querystring to use to select the devices from netbox", env)]
    netbox_device_filter: String,

    #[structopt(short, long, help = "Check mode, will not push any change to Netshot")]
    check: bool,

}

/// Main application entrypoint
#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let opt = Opt::from_args();
    let mut logging_level = LevelFilter::Info;
    if opt.debug {
        logging_level = LevelFilter::Debug;
    }

    SimpleLogger::new()
        .with_level(logging_level)
        .init()
        .unwrap();

    log::info!("Logger initialized with level {}", logging_level);
    log::debug!("CLI Parameters : {:#?}", opt);

    let netbox_client = netbox::NetboxClient::new(opt.netbox_url, opt.netbox_token).unwrap();
    let netbox_ping = netbox_client.ping().await.unwrap();

    let netbox_devices = netbox_client.get_devices(opt.netbox_device_filter).await.unwrap();

    Ok(())
}
