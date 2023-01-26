#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[macro_use]
extern crate log;

use bme::*;
use bme68x_rust::{Device, DeviceConfig, Filter, GasHeaterConfig, Interface, Odr};
use chrono::{Local, NaiveDateTime};
use dotenvy::dotenv;
use std::cmp::max;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::{fs, env};
mod bme;
mod bsec;
mod graphite;

fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().expect(".env file not found");
    for (key, value) in env::vars() {
        debug!("{key}: {value}");
    }

    let mut run_loop = true;

    let (tx, rx) = channel();

    ctrlc::set_handler(move || {
        let serialized_state = bsec::get_bsec_state();

        fs::write("last_state.bin", serialized_state)
            .map(|_| info!("BSEC state saved."))
            .unwrap_or_else(|_| warn!("Error saving BSEC state."));

        tx.send(0)
            .unwrap_or_else(|_| error!("Failed to signal termination."));
    })
    .expect("Error setting Ctrl-C handler");

    let mut bme: Option<Device<I2cDriver>> = None;

    for i in 0..9 {
        let pathstring = format!("/dev/i2c-{}", i);
        let path = Path::new(&pathstring);
        match path.try_exists() {
            Ok(result) => {
                if result {
                    info!("Found i2c Device on {}", path.display());
                    let driver = bme::create_device(path);
                    bme = Some(bme::init(driver));
                    break;
                }
            }
            Err(_) => {}
        }
    }

    let mut bme = bme.expect("Cannot find i2c Device.");

    let graphite_url = env::var("GRAHITE_URL").unwrap_or(String::from("default"));

    let mut graphite_state =
        graphite::init(graphite_url.as_str()).expect("Failed to connect to graphite");

    let mut bsec_state = bsec::State::default();

    bsec::get_version(&mut bsec_state);

    bsec::init(&mut bsec_state);

    let last_state = fs::read("last_state.bin").ok();

    match last_state {
        Some(serialized_state) => {
            bsec::set_bsec_state(serialized_state);
        }
        None => {
            info!("Last BSEC state not found.");
        }
    }

    bsec::update_subscription(&mut bsec_state, BSEC_SAMPLE_RATE_LP as f32);

    while run_loop {
        let start_timestamp = Local::now().timestamp_nanos();

        info!("Calling at:     {}", Local::now());

        bsec::get_sensor_config(&mut bsec_state, start_timestamp);

        bme.set_config(
            DeviceConfig::default()
                .filter(Filter::Size3)
                .odr(Odr::StandbyNone)
                .oversample_humidity(bsec_state.sensor_settings.humidity_oversampling.into())
                .oversample_temperature(bsec_state.sensor_settings.temperature_oversampling.into())
                .oversample_pressure(bsec_state.sensor_settings.pressure_oversampling.into()),
        )
        .expect("failed setting config");

        let heater_config = GasHeaterConfig::default()
            .enable()
            .heater_temp(bsec_state.sensor_settings.heater_temperature)
            .heater_duration(bsec_state.sensor_settings.heater_duration)
            .heater_temp_profile(
                bsec_state
                    .sensor_settings
                    .heater_duration_profile
                    .as_mut_ptr(),
            )
            .heater_dur_profile(
                bsec_state
                    .sensor_settings
                    .heater_duration_profile
                    .as_mut_ptr(),
            );

        bme.set_gas_heater_conf(bsec_state.sensor_settings.op_mode.into(), heater_config)
            .expect("failed setting heater config");

        // -------------------------------------------------------

        if bsec_state.sensor_settings.trigger_measurement == 1 {
            bme.set_op_mode(bsec_state.sensor_settings.op_mode.into())
                .expect("Failed setting operation mode");

            let delay_period = bme.get_measure_duration(bsec_state.sensor_settings.op_mode.into());
            bme.interface.delay(delay_period);

            let mut measure_results = bme.get_data(bsec_state.sensor_settings.op_mode.into());

            loop {
                if measure_results.is_ok() && // no new data
                 measure_results.as_ref().unwrap()[0].status & 0b10000 == 0b10000 && // heater stable
                    measure_results.as_ref().unwrap()[0].status & 0b100000 == 0b100000
                // gas measurement valid
                {
                    break;
                }

                bme.interface.delay(1000);

                measure_results = bme.get_data(bsec_state.sensor_settings.op_mode.into());
            }

            let measure_results = measure_results.unwrap();

            debug!("{:#?}", measure_results[0]);

            let sensor_inputs =
                bsec::process_data(&bsec_state, &measure_results[0], start_timestamp);

            debug!("{:?}", sensor_inputs);

            let sensor_outputs = bsec::do_steps(&mut bsec_state, &sensor_inputs);

            let metrics_string = graphite::build_output(sensor_outputs, start_timestamp);

            graphite::send_metrics(&mut graphite_state, metrics_string)
                .err()
                .map(|e| {
                    error!("Failed to send metrics: {}", e);
                    loop {
                        match graphite_state.reconnect() {
                            Ok(_) => {
                                break;
                            }
                            Err(_) => spin_sleep::sleep(Duration::from_micros(500000)),
                        }
                    }
                });
        }

        // ---------------------------------------------

        let next_call = bsec_state.sensor_settings.next_call;

        info!(
            "Next call time: {}",
            NaiveDateTime::from_timestamp_opt(
                next_call / 1000 / 1000 / 1000,
                (next_call % 1000000000) as u32
            )
            .unwrap()
            .and_local_timezone(Local::now().timezone())
            .unwrap()
        );

        let wait_time = max(1, (next_call - Local::now().timestamp_nanos()) / 1000 - 200);
        info!("Sleeping for: {} ms", wait_time / 1000);

        spin_sleep::sleep(Duration::from_micros(wait_time as u64));

        rx.try_recv()
            .and_then(|_| {
                run_loop = false;
                Ok(())
            })
            .ok();
    }

    Ok(())
}

fn print_result(result: i32, op_name: &str) {
    if result == bsec_library_return_t::BSEC_OK {
        debug!("BSEC {}: OK", op_name);
    } else {
        error!("BSEC {}: Error {}", op_name, result);
    }
}
