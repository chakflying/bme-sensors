use crate::*;

use chrono::Local;

#[derive(Default)]
pub struct State {
    pub result: i32,
    pub requested_virtual_sensors: Vec<bsec_sensor_configuration_t>,
    pub required_sensor_settings: Vec<bsec_sensor_configuration_t>,
    pub n_required_sensor_settings: u8,
    pub sensor_settings: bsec_bme_settings_t,
}

pub fn get_version(state: &mut State) -> bsec_version_t {
    let mut version = bsec_version_t {
        major: 0,
        minor: 0,
        major_bugfix: 0,
        minor_bugfix: 0,
    };

    unsafe {
        state.result = bsec_get_version(&mut version as *mut bsec_version_t);
    }

    println!(
        "BSEC Version: {}.{}.{}",
        version.major, version.minor, version.major_bugfix
    );

    return version;
}

pub fn init(state: &mut State) {
    unsafe {
        state.result = bsec_init();
    }

    print_result(state, "Init");
}

pub fn update_subscription(state: &mut State) {
    state.requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_STATIC_IAQ as u8,
    });

    state.requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_TEMPERATURE as u8,
    });

    state.requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_HUMIDITY as u8,
    });

    state.requested_virtual_sensors.push(bsec_sensor_configuration_t {
        sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
        sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_STABILIZATION_STATUS as u8,
    });

    for _ in 0..BSEC_MAX_PHYSICAL_SENSOR {
        state
            .required_sensor_settings
            .push(bsec_sensor_configuration_t {
                sample_rate: 0f32,
                sensor_id: 1,
            })
    }

    state.n_required_sensor_settings = BSEC_MAX_PHYSICAL_SENSOR as u8;

    unsafe {
        state.result = bsec_update_subscription(
            state.requested_virtual_sensors.as_mut_ptr(),
            state.requested_virtual_sensors.len() as u8,
            state.required_sensor_settings.as_mut_ptr(),
            &mut state.n_required_sensor_settings as *mut u8,
        );
    }

    print_result(state, "Update Subscription");
}

pub fn get_sensor_config(state: &mut State) {
    unsafe {
        state.result = bsec_sensor_control(
            Local::now().timestamp_nanos(),
            &mut state.sensor_settings as *mut bsec_bme_settings_t,
        );
    }

    print_result(state, "Sensor Control");
    println!("{:?}", state.sensor_settings);
}

pub fn do_steps(state: &mut State, inputs: &Vec<bsec_input_t>) {
    let mut sensor_outputs = Vec::new();
    let mut n_sensor_outputs: u8 = state.requested_virtual_sensors.len() as u8;

    for _ in 0..state.requested_virtual_sensors.len() {
        sensor_outputs.push(bsec_output_t::default());
    }

    unsafe {
        state.result = bsec_do_steps(
            inputs.as_ptr(),
            state.n_required_sensor_settings,
            sensor_outputs.as_mut_ptr(),
            &mut n_sensor_outputs as *mut u8,
        );
    }

    print_result(state, "Do Steps");

    for i in 0..n_sensor_outputs as usize {
        println!("{:?}", sensor_outputs.get(i));
    }
}

fn print_result(state: &State, op_name: &str) {
    if state.result == 0 {
        println!("BSEC {}: OK", op_name);
    } else {
        println!("BSEC {}: Error {}", op_name, state.result);
    }
}