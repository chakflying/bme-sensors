use bme68x_rust::{CommInterface, Device, Error as BmeError, Interface};
use linux_embedded_hal::i2cdev::core::{I2CDevice, I2CTransfer};
use linux_embedded_hal::i2cdev::linux::I2CMessage;
use linux_embedded_hal::I2cdev;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct I2cDriver {
    pub path: PathBuf,
    pub device: I2cdev,
}

pub fn create_device(path: &Path) -> I2cDriver {
    let mut device = I2cdev::new(path).expect("Failed to create device");
    device
        .set_slave_address(0x77)
        .expect("Cannot set device address");
    I2cDriver {
        path: path.to_path_buf(),
        device: device,
    }
}

pub fn init(driver: I2cDriver) -> Device<I2cDriver> {
    Device::initialize(driver).expect("Cannot initialize device")
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
        let write_message = I2CMessage::write(&[_reg_addr]);
        let read_message = I2CMessage::read(_reg_data);
        self.device
            .transfer(&mut [write_message, read_message])
            .map(|_| ())
            .map_err(|err| {
                println!("{}: {:#?}", err, err.source());
                BmeError::CommunicationFailure
            })
    }

    fn write(&mut self, _reg_addr: u8, _reg_data: &[u8]) -> Result<(), BmeError> {
        self.device
            .smbus_write_i2c_block_data(_reg_addr, _reg_data)
            .map_err(|err| {
                println!("{}: {:#?}", err, err.source());
                BmeError::CommunicationFailure
            })
    }
}
