#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use bme::*;
use chrono::Local;
use std::path::Path;
mod bme;
mod bsec;

fn main() -> std::io::Result<()> {
    println!("Hello World!");
    let time_now = Local::now().naive_local();
    println!("Time now is: {}", time_now);

    let mut bme_state = bme::State::default();

    for i in 0..9 {
        let pathstring = format!("/dev/i2c-{}", i);
        let path = Path::new(&pathstring);
        match path.try_exists() {
            Ok(result) => {
                if result {
                    println!("Found i2c Device on {}", path.display());
                    let driver = bme::create_device(path);
                    bme_state = bme::init(driver);
                    break;
                }
            }
            Err(_) => {}
        }
    }

    if bme_state.driver.is_none() {
        println!("Cannot find i2c Device.");
    }

    let mut bsec_state = bsec::State::default();

    bsec::get_version(&mut bsec_state);

    bsec::init(&mut bsec_state);

    bsec::update_subscription(&mut bsec_state);

    bsec::get_sensor_config(&mut bsec_state);

    let timestamp = Local::now().timestamp_nanos();

    let mut sensor_inputs = Vec::new();

    for i in 0..bsec_state.n_required_sensor_settings as usize {
        sensor_inputs.push(bsec_input_t {
            time_stamp: timestamp,
            signal: 0f32,
            signal_dimensions: 1,
            sensor_id: bsec_state
                .required_sensor_settings
                .get(i)
                .unwrap()
                .sensor_id,
        });
    }

    bsec::do_steps(&mut bsec_state, &sensor_inputs);

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
