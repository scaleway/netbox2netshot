# Netbox2Netshot

> Still in early development

## Introduction

Netbox2Netshot is a tool that allows you to synchronize your Netbox DCIM (using specific criterias) to Netshot so your devices would automatically get backed up by Netshot once added in Netbox.

The tool is coded in Rust and doesn't required any dependency installed

## How to use

### Installation

Gather a pre-built binary or install it using Cargo

```bash
cargo install netbox2netshot
```

### Parameters

Most parameters can be set either via command line arguments or environment variables

```bash
netbox2netshot 0.0.1
Synchronization tool between netbox and netshot

USAGE:
    netbox2netshot [FLAGS] [OPTIONS] --netbox-token <netbox-token> --netbox-url <netbox-url> --netshot-url <netshot-url>

FLAGS:
    -c, --check      Check mode, will not push any change to Netshot
    -d, --debug      Enable debug/verbose mode
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --netbox-devices-filter <netbox-devices-filter>
            The querystring to use to select the devices from netbox [env: NETBOX_DEVICES_FILTER=]  [default: ]

        --netbox-token <netbox-token>                      The Netbox token [env: NETBOX_TOKEN]
        --netbox-url <netbox-url>                          The Netbox API URL [env: NETBOX_URL=]
        --netshot-token <netshot-token>                    The Netshot token [env: NETSHOT_TOKEN]  [default: ]
        --netshot-url <netshot-url>                        The Netshot API URL [env: NETSHOT_URL=]

```

The query-string format need to be like this (url query string without the `?`):

```bash
status=active&platform=cisco-ios&platform=cisco-ios-xe&platform=cisco-ios-xr&platform=cisco-nx-os&platform=juniper-junos&has_primary_ip=true&tenant_group=network
```

