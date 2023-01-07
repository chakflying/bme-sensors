#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use chrono::Local;
mod bme;

fn main() -> std::io::Result<()> {
    println!("Hello World!");
    let time_now = Local::now().naive_local();
    println!("Time now is: {}", time_now);

    let mut version = bsec_version_t {
        major: 0,
        minor: 0,
        major_bugfix: 0,
        minor_bugfix: 0,
    };

    let mut result = bsec_library_return_t::BSEC_OK;

    unsafe {
        bsec_get_version(&mut version as *mut bsec_version_t);
    }

    println!(
        "BSEC Version: {}.{}.{}",
        version.major, version.minor, version.major_bugfix
    );

    unsafe {
        result = bsec_init();
    }

    print_result(result, "Init");

    let mut requested_virtual_sensors: Vec<bsec_sensor_configuration_t> = Vec::new();

    requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_STATIC_IAQ as u8,
    });

    requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_TEMPERATURE as u8,
    });

    requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_HUMIDITY as u8,
    });

    requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_STABILIZATION_STATUS as u8,
    });

    let mut required_sensor_settings: Vec<bsec_sensor_configuration_t> = Vec::new();

    for _ in 0..BSEC_MAX_PHYSICAL_SENSOR {
        required_sensor_settings.push(bsec_sensor_configuration_t {
            sample_rate: 0f32,
            sensor_id: 1,
        })
    }

    let mut n_required_sensor_settings: u8 = BSEC_MAX_PHYSICAL_SENSOR as u8;

    unsafe {
        result = bsec_update_subscription(
            requested_virtual_sensors.as_mut_ptr(),
            requested_virtual_sensors.len() as u8,
            required_sensor_settings.as_mut_ptr(),
            &mut n_required_sensor_settings as *mut u8,
        );
    }

    print_result(result, "Update Subscription");

    let mut sensor_settings: bsec_bme_settings_t = bsec_bme_settings_t::default();

    unsafe {
        result = bsec_sensor_control(
            Local::now().timestamp_nanos(),
            &mut sensor_settings as *mut bsec_bme_settings_t,
        );
    }

    print_result(result, "Sensor Control");

    println!("{:?}", sensor_settings);

    let timestamp = Local::now().timestamp_nanos();

    let mut sensor_inputs = Vec::new();

    for i in 0..n_required_sensor_settings as usize {
        sensor_inputs.push(bsec_input_t {
            time_stamp: timestamp,
            signal: 0f32,
            signal_dimensions: 1,
            sensor_id: required_sensor_settings.get(i).unwrap().sensor_id,
        });
    }

    let mut sensor_outputs = Vec::new();
    let mut n_sensor_outputs: u8 = requested_virtual_sensors.len() as u8;

    for _ in 0..requested_virtual_sensors.len() {
        sensor_outputs.push(bsec_output_t::default());
    }

    unsafe {
        result = bsec_do_steps(
            sensor_inputs.as_mut_ptr(),
            n_required_sensor_settings,
            sensor_outputs.as_mut_ptr(),
            &mut n_sensor_outputs as *mut u8,
        );
    }

    print_result(result, "Do Steps");

    for i in 0..n_sensor_outputs as usize {
        println!("{:?}", sensor_outputs.get(i));
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
