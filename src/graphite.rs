use crate::bsec::{bsec_virtual_sensor_t, bsec_output_t};
use log::debug;
use std::{
    io::{Error, ErrorKind, Write},
    net::TcpStream,
};

#[derive(Default)]
pub struct State {
    url: String,
    connection: Option<TcpStream>,
}

impl State {
    pub fn reconnect(&mut self) -> Result<(), Error> {
        debug!("Trying to reconnect...");
        let connection = TcpStream::connect(self.url.as_str())?;
        connection.set_nodelay(true)?;
        connection.set_nonblocking(true)?;
        self.connection = Some(connection);
        Ok(())
    }
}

pub fn init(url: &str) -> State {
    if let Ok(connection) = TcpStream::connect(url) {
        let _ = connection.set_nodelay(true);
        let _ = connection.set_nonblocking(true);
        return State {
            url: String::from(url),
            connection: Some(connection),
        };
    } else {
        return State {
                url: String::from(url),
                connection: None,
            }
    }
}

pub fn build_output(sensor_outputs: Vec<bsec_output_t>, timestamp: i64) -> String {
    let mut metrics_string = String::from("");

    for sensor in sensor_outputs {
        let metric_name = match sensor.sensor_id as u32 {
            bsec_virtual_sensor_t::BSEC_OUTPUT_STATIC_IAQ => "study.iaq",
            bsec_virtual_sensor_t::BSEC_OUTPUT_STABILIZATION_STATUS => "study.stable",
            bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_TEMPERATURE => {
                "study.temperature"
            }
            bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_HUMIDITY => "study.humidity",
            bsec_virtual_sensor_t::BSEC_OUTPUT_RAW_PRESSURE => "study.pressure",
            bsec_virtual_sensor_t::BSEC_OUTPUT_BREATH_VOC_EQUIVALENT => "study.voc",
            bsec_virtual_sensor_t::BSEC_OUTPUT_RAW_GAS => "study.gas_resistance",
            _ => "study.unknown",
        };
        metrics_string.push_str(&*format!(
            "{} {} {}\n",
            metric_name,
            sensor.signal,
            timestamp / 1000 / 1000 / 1000
        ));
    }

    metrics_string
}

pub fn send_metrics(state: &mut State, metrics: &str) -> Result<(), Error> {
    match state.connection.as_mut() {
        Some(connection) => {
            connection.write(metrics.as_bytes())?;
            connection.flush()?;
        }
        None => {
            return Err(Error::new(ErrorKind::Other, "not connected"));
        }
    }
    Ok(())
}
