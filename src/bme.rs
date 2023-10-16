use bme68x_rust::{CommInterface, Device, Error as BmeError, Interface};
use embedded_hal::i2c::blocking::I2c;
use linux_embedded_hal::I2cdev;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct I2cDriver {
    pub path: PathBuf,
    pub device: I2cdev,
}

pub fn create_device(path: &Path) -> I2cDriver {
    let mut device = I2cdev::new(path).expect("Failed to create device");
    device
        .set_slave_address(bme68x_rust::I2C_ADDR_HIGH.into())
        .expect("Cannot set device address");
    I2cDriver {
        path: path.to_path_buf(),
        device: device,
    }
}

pub fn init(driver: I2cDriver) -> Option<Device<I2cDriver>> {
    match Device::initialize(driver) {
        Ok(device) => return Some(device),
        Err(e) => {
            error!("{:?}", e);
            return None;
        }
    }
}

impl Interface for I2cDriver {
    fn interface_type(&self) -> CommInterface {
        CommInterface::I2C
    }

    fn delay(&self, period: u32) {
        let delay = Duration::from_micros(period as u64);
        spin_sleep::sleep(delay);
    }

    fn read(&mut self, _reg_addr: u8, _reg_data: &mut [u8]) -> Result<(), BmeError> {
        // Send the address to start reading, then read
        self.device
            .write_read(bme68x_rust::I2C_ADDR_HIGH, &[_reg_addr], _reg_data)
            .map_err(|err| {
                println!("{:#?}", err);
                BmeError::CommunicationFailure
            })
    }

    fn write(&mut self, _reg_addr: u8, _reg_data: &[u8]) -> Result<(), BmeError> {
        // Send pairs of [address, data] for writing
        let mut bytes = Vec::with_capacity(_reg_data.len() * 2);
        for (i, b) in _reg_data.iter().enumerate() {
            bytes.push(_reg_addr + i as u8);
            bytes.push(b.to_owned());
        }

        self.device
            .write(bme68x_rust::I2C_ADDR_HIGH, bytes.as_slice())
            .map_err(|err| {
                println!("{:#?}", err);
                BmeError::CommunicationFailure
            })
    }
}
