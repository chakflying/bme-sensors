# BME688 Data Logger

This combines the BSEC library from Bosch and `bme68x-rust` to read and process data from the BME688 sensor, and send it to a graphite server. The project runs on a Raspberry Pi 4B with a armv7l 32-bit OS, with the sensor on a Adafruit breakout board connected with I2C.

## Hardware

- Raspberry Pi 4B
- Adafruit BME688
- STEMMA QT / Qwiic JST SH 4-pin Cable with Premium Female Sockets

## Dependencies

- Bosch BSEC Library `2.2.0`

  Download from the [Bosch website](https://www.bosch-sensortec.com/software-tools/software/bme688-software/#Library). 

  Located in the dowloaded zip archive, `algo/normal_version/bin/RaspberryPi/PiThree_ArmV6/`, copy the files 

  - `bsec_datatypes.h`, 
  - `bsec_interface.h`,
  - `libalgobsec.a` 

  into the project folder `lib/`.

## Configuration

The Graphite server location is configured via environment variables. You can also create file `.env` in the project root, and input the graphite server location:

```shell
GRAPHITE_URL=<your-graphite-server>:2003
```

## Usage

Build the program in release mode:

```shell
cargo build --release
```

Then the CLI program can be run in the background with:

```shell
nohup {project directory}/target/release/bme-sensors & disown
```

You can also setup a systemd service such that it runs on startup.

```shell
[Unit]
Description=BME Sensors service
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=1
User=<current-user>
WorkingDirectory=/<project directory>
ExecStart=/<project directory>/target/release/bme-sensors

[Install]
WantedBy=multi-user.target
```
