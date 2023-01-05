#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// use regex::Regex;
use chrono::{Local};

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

    let mut result: bsec_library_return_t = 0;

    unsafe {
        bsec_get_version(&mut version as *mut bsec_version_t);
    }

    println!("BSEC Version: {}.{}.{}", version.major, version.minor, version.major_bugfix);

    unsafe {
        result = bsec_init();
    }

    println!("BSEC init: {}", if result == 0 { "OK" } else { "Error" });

    Ok(())
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
                .map(|u| if u == 0 { None } else { Some(Rc::clone(&self.buf)) })
                .transpose()
        }
    }
}
