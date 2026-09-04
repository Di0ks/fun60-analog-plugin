# MonsGeek FUN60 Ultra analog keyboard plugin for Linux

A userspace library for simple communication to MonsGeek FUN60 Ultra keyboard, including reading raw depth values on Linux.
Provides a [Wooting Analog SDK](https://github.com/WootingKb/wooting-analog-sdk) plugin implementation, as well as a non-SDK way to subscribe to keyboard events.

Requires the `monsgeek-akko-linux` driver with UDS server support to be installed ([my fork](https://github.com/Di0ks/monsgeek-akko-linux/tree/fun60-uds-depth-server)).

## Installation steps

### 1. Driver

Prior to installing the library, you need to install the keyboard driver capable of providing a UDS server compatible with the protocol descibed in [socket.rs](src/socket.rs). Follow the installation steps [here](https://github.com/Di0ks/monsgeek-akko-linux/tree/fun60-uds-depth-server) to get one.

After installation you should be able to run this:
```bash
$ iot_driver depthd -s /tmp/test.sock
Depth server listening on /tmp/test.sock
```
(You can stop the server after this check)

### 2. Plugin

To use as a Wooting Analog SDK plugin firstly ensure you followed the steps to [install the SDK](https://github.com/WootingKb/wooting-analog-sdk/blob/develop/docs/INSTALL.md).

After the SDK is installed, clone this repo and run the installation script:
```bash
$ git clone https://github.com/Di0ks/fun60-analog-plugin
$ cd fun60-analog-plugin
$ chmod u+x ./install.sh && ./install.sh
```
You will be prompted for a root password in order to copy the library to the SDK plugin dir.

### Uninstallation

To uninstall the plugin run the installation script from above with `--uninstall` flag (also requires root privileges):
```bash
$ ./install.sh --uninstall
```

## Usage

The library requires a running UDS depth server either at the default path (`/run/iot_driver/depth.sock`) or at the path specified by the `IOT_DRIVER_DEPTH_SOCK` environmental variable.
Also ensure your user is in the `input` group to access the socket at its default path without root privileges.

### As a plugin

When using this library as a Wooting Analog SDK plugin it's unlikely that applications using the SDK will start the depth server automatically, so you need to do it manually before starting the application:
```bash
sudo iot_driver depthd
```

### As a Rust library

To use this library as a Rust crate add the following line to `Cargo.toml` under `[dependencies]`:
```toml
fun60_analog_plugin = { git = "https://github.com/Di0ks/fun60-analog-plugin" }
```

If implementing an application using this library as a direct dependency, it's recommended to check whether the UDS socket exists and can be accessed. If not, the application can start a server on a non-root path (e.g. `iot_driver depthd -s /tmp/depth.sock`) and connect to it.

## Documentation

A documentation for this library can be compiled using `cargo`.
If used as a dependency:
```bash
cargo doc --package fun60-analog-plugin
```
From the project directory:
```bash
cargo doc --no-deps
```

## License
This project is licensed under the **MIT License**.

See [LICENSE](LICENSE) for details.