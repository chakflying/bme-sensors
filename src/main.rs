#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use bme::*;
use bme68x_rust::{Device, DeviceConfig, Filter, GasHeaterConfig, Interface, Odr, OperationMode};
use chrono::{DateTime, Local, NaiveDateTime};
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
mod bme;
mod bsec;

fn main() -> std::io::Result<()> {
    println!("Hello World!");
    let time_now = Local::now().naive_local();
    println!("Time now is: {}", time_now);

    let mut run_loop = true;

    let (tx, rx) = channel();

    ctrlc::set_handler(move || {
        let serialized_state = bsec::get_bsec_state();

        fs::write("last_state.bin", serialized_state)
            .map(|_| println!("BSEC state saved."))
            .unwrap_or_else(|_| println!("Error saving BSEC state."));

        tx.send(0)
            .unwrap_or_else(|_| println!("Failed to signal termination."));
    })
    .expect("Error setting Ctrl-C handler");

    let mut bme: Option<Device<I2cDriver>> = None;

    for i in 0..9 {
        let pathstring = format!("/dev/i2c-{}", i);
        let path = Path::new(&pathstring);
        match path.try_exists() {
            Ok(result) => {
                if result {
                    println!("Found i2c Device on {}", path.display());
                    let driver = bme::create_device(path);
                    bme = Some(bme::init(driver));
                    break;
                }
            }
            Err(_) => {}
        }
    }

    let mut bme = bme.expect("Cannot find i2c Device.");

    let mut bsec_state = bsec::State::default();

    bsec::get_version(&mut bsec_state);

    bsec::init(&mut bsec_state);

    let last_state = fs::read("last_state.bin").ok();
    
    match last_state {
        Some(serialized_state) => {
            bsec::set_bsec_state(serialized_state);
        }
        None => {
            println!("Last BSEC state not found.");

        }
    }

    bsec::update_subscription(&mut bsec_state);

    while run_loop {
        let start_timestamp = Local::now().timestamp_nanos();

        println!("Calling at: {}", Local::now());

        bsec::get_sensor_config(&mut bsec_state, start_timestamp);

        bme.set_config(
            DeviceConfig::default()
                .filter(Filter::Size3)
                .odr(Odr::StandbyNone)
                .oversample_humidity(unsafe {
                    std::mem::transmute(bsec_state.sensor_settings.humidity_oversampling)
                })
                .oversample_temperature(unsafe {
                    std::mem::transmute(bsec_state.sensor_settings.temperature_oversampling)
                })
                .oversample_pressure(unsafe {
                    std::mem::transmute(bsec_state.sensor_settings.pressure_oversampling)
                }),
        )
        .expect("failed setting config");

        let mut heater_config = GasHeaterConfig::default()
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

        let shared_duration = bme.get_measure_duration(OperationMode::Forced) as u16 / 1000;

        heater_config = heater_config.heater_shared_duration(140 - shared_duration);

        bme.set_gas_heater_conf(OperationMode::Forced, heater_config)
            .expect("failed setting heater config");

        // -------------------------------------------------------

        bme.set_op_mode(OperationMode::Forced)
            .expect("Failed setting operation mode");

        let delay_period = bme.get_measure_duration(OperationMode::Forced)
            + ((140 - shared_duration as u32) * 1000)
            + 750000;
        bme.interface.delay(delay_period);

        let measure_results = bme
            .get_data(OperationMode::Forced)
            .expect("Failed getting measure results");

        println!("{:?}", measure_results);

        let mut sensor_inputs = Vec::new();

        for i in 0..bsec_state.n_required_sensor_settings as usize {
            let sensor_id = bsec_state
                .required_sensor_settings
                .get(i)
                .unwrap()
                .sensor_id;

            let signal = match sensor_id as u32 {
                bsec_physical_sensor_t::BSEC_INPUT_PRESSURE => measure_results[0].pressure,
                bsec_physical_sensor_t::BSEC_INPUT_HUMIDITY => measure_results[0].humidity,
                bsec_physical_sensor_t::BSEC_INPUT_TEMPERATURE => measure_results[0].temperature,
                bsec_physical_sensor_t::BSEC_INPUT_GASRESISTOR => measure_results[0].gas_resistance,
                bsec_physical_sensor_t::BSEC_INPUT_HEATSOURCE => 5f32,
                bsec_physical_sensor_t::BSEC_INPUT_PROFILE_PART => {
                    measure_results[0].gas_index.into()
                }
                _ => 0f32,
            };

            sensor_inputs.push(bsec_input_t {
                time_stamp: start_timestamp,
                signal: signal,
                signal_dimensions: 1,
                sensor_id: sensor_id,
            });
        }

        bsec::do_steps(&mut bsec_state, &sensor_inputs);

        // ---------------------------------------------

        let next_call = bsec_state.sensor_settings.next_call;
        let wait_time = next_call - Local::now().timestamp_nanos();

        // println!(
        //     "Next call time: {}",
        //     NaiveDateTime::from_timestamp_opt(
        //         next_call / 1000 / 1000 / 1000,
        //         (next_call % 1000000000) as u32
        //     )
        //     .unwrap()
        //     .and_local_timezone(Local::now().timezone())
        //     .unwrap()
        // );
        println!("Sleeping for: {} ms", wait_time / 1000 / 1000);

        bme.interface.delay(wait_time as u32 / 1000 - 500);

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
        println!("BSEC {}: OK", op_name);
    } else {
        println!("BSEC {}: Error {}", op_name, result);
    }
}

mod my_reader {
    use std::{
        fs::File,
        io::{self, prelude::*},
        rc::Rc,
    };

    pub struct BufReader {
        reader: io::BufReader<File>,
        buf: Rc<String>,
    }

    fn new_buf() -> Rc<String> {
        Rc::new(String::with_capacity(1024)) // Tweakable capacity
    }

    impl BufReader {
        pub fn open(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
            let file = File::open(path)?;
            let reader = io::BufReader::new(file);
            let buf = new_buf();

            Ok(Self { reader, buf })
        }
    }

    impl Iterator for BufReader {
        type Item = io::Result<Rc<String>>;

        fn next(&mut self) -> Option<Self::Item> {
            let buf = match Rc::get_mut(&mut self.buf) {
                Some(buf) => {
                    buf.clear();
                    buf
                }
                None => {
                    self.buf = new_buf();
                    Rc::make_mut(&mut self.buf)
                }
            };

            self.reader
                .read_line(buf)
                .map(|u| {
                    if u == 0 {
                        None
                    } else {
                        Some(Rc::clone(&self.buf))
                    }
                })
                .transpose()
        }
    }
}
