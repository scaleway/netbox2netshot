# Netbox2Netshot

## Introduction

Netbox2Netshot is a tool that allows you to synchronize your [Netbox DCIM](https://github.com/netbox-community/netbox) (using specific criterias) to [Netshot](https://github.com/netfishers-onl/Netshot)  so your devices would automatically get backed up by Netshot once added in Netbox.

The tool is coded in Rust and doesn't required any runtime dependency installed

## How to use

### Installation

Gather a pre-built binary or install it using Cargo

```bash
cargo install netbox2netshot
```

### Parameters

Most parameters can be set either via command line arguments or environment variables

```bash
netbox2netshot [FLAGS] [OPTIONS] --netbox-url <netbox-url> --netshot-domain-id <netshot-domain-id> --netshot-token <netshot-token> --netshot-url <netshot-url>

FLAGS:
    -c, --check      Check mode, will not push any change to Netshot
    -d, --debug      Enable debug/verbose mode
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --netbox-devices-filter <netbox-devices-filter>
            The querystring to use to select the devices from netbox [env: NETBOX_DEVICES_FILTER=]  [default: ]

        --netbox-token <netbox-token>                      The Netbox token [env: NETBOX_TOKEN]  [default: ]
        --netbox-url <netbox-url>                          The Netbox API URL [env: NETBOX_URL=]
        --netbox-vms-filter <netbox-vms-filter>
            The querystring to use to select the VM from netbox [env: NETBOX_VMS_FILTER=]

        --netshot-domain-id <netshot-domain-id>
            The domain ID to use when importing a new device [env: NETSHOT_DOMAIN_ID=]

        --netshot-token <netshot-token>                    The Netshot token [env: NETSHOT_TOKEN]
        --netshot-url <netshot-url>                        The Netshot API URL [env: NETSHOT_URL=]
```

The query-string format need to be like this (url query string without the `?`):

```bash
status=active&platform=cisco-ios&platform=cisco-ios-xe&platform=cisco-ios-xr&platform=cisco-nx-os&platform=juniper-junos&has_primary_ip=true&tenant_group=network
```

