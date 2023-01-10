use crate::*;
use bme68x_rust::{CommInterface, Device, DeviceConfig, Error, Interface};
use embedded_hal::blocking::i2c::{Read, Write};
use linux_embedded_hal::I2cdev;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Default)]
pub struct State {
    pub driver: Option<Device<I2cDriver>>,
    pub result: i8,
}

pub struct I2cDriver {
    pub path: PathBuf,
    pub device: I2cdev,
}

pub fn create_device(path: &Path) -> I2cDriver {
    let device = I2cdev::new(path).expect("Failed to create device");
    I2cDriver {
        path: path.to_path_buf(),
        device: device,
    }
}

pub fn init(driver: I2cDriver) -> State {
    let bme = Device::initialize(driver).expect("Cannot initialize device");
    State {
        driver: Some(bme),
        result: 0,
    }
}

impl Interface for I2cDriver {
    fn interface_type(&self) -> CommInterface {
        CommInterface::I2C
    }

    fn delay(&self, period: u32) {
        let delay = Duration::from_micros(period as u64);
        std::thread::sleep(delay);
    }

    fn read(&mut self, _reg_addr: u8, _reg_data: &mut [u8]) -> Result<(), Error> {
        self.device
            .read(_reg_addr, _reg_data)
            .map_err(|_| Error::CommunicationFailure)
    }

    fn write(&mut self, _reg_addr: u8, _reg_data: &[u8]) -> Result<(), Error> {
        self.device
            .write(_reg_addr, _reg_data)
            .map_err(|_| Error::CommunicationFailure)
    }
}

fn print_result(state: State, op_name: &str) {
    if state.result == 0 {
        println!("BME {}: OK", op_name);
    } else {
        println!("BME {}: Error {}", op_name, state.result);
    }
}
