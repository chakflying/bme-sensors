use std::{
    io::{Error, Write},
    net::TcpStream,
};

#[derive(Default)]
pub struct State {
    connection: Option<TcpStream>,
}

impl State {
    pub fn reconnect(&mut self) -> Result<(), Error> {
        println!("Trying to reconnect...");
        let connection = TcpStream::connect("bme-sensors-logging.fly.dev:2003")?;
        connection.set_nodelay(true)?;
        connection.set_nonblocking(true)?;
        self.connection = Some(connection);
        Ok(())
    }
}

pub fn init() -> Result<State, Error> {
    let connection = TcpStream::connect("bme-sensors-logging.fly.dev:2003")?;
    connection.set_nodelay(true)?;
    connection.set_nonblocking(true)?;
    Ok(State {
        connection: Some(connection),
    })
}

pub fn send_metrics(state: &mut State, metrics: String) -> Result<(), Error> {
    match state.connection.as_mut() {
        Some(connection) => {
            connection.write(metrics.as_bytes())?;
        }
        None => {}
    }
    Ok(())
}
