use crate::*;

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
    state
        .requested_virtual_sensors
        .push(bsec_sensor_configuration_t {
            sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
            sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_STATIC_IAQ as u8,
        });

    state
        .requested_virtual_sensors
        .push(bsec_sensor_configuration_t {
            sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
            sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_TEMPERATURE as u8,
        });

    state
        .requested_virtual_sensors
        .push(bsec_sensor_configuration_t {
            sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
            sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_HUMIDITY as u8,
        });

    state
        .requested_virtual_sensors
        .push(bsec_sensor_configuration_t {
            sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
            sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_STABILIZATION_STATUS as u8,
        });

    state
        .requested_virtual_sensors
        .push(bsec_sensor_configuration_t {
            sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
            sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_RAW_PRESSURE as u8,
        });

    state
        .requested_virtual_sensors
        .push(bsec_sensor_configuration_t {
            sample_rate: BSEC_SAMPLE_RATE_CONT as f32,
            sensor_id: bsec_virtual_sensor_t::BSEC_OUTPUT_BREATH_VOC_EQUIVALENT as u8,
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

pub fn get_sensor_config(state: &mut State, timestamp: i64) {
    unsafe {
        state.result = bsec_sensor_control(
            timestamp,
            &mut state.sensor_settings as *mut bsec_bme_settings_t,
        );
    }

    print_result(state, "Sensor Control");
    // println!("{:?}", state.sensor_settings);
}

pub fn do_steps(state: &mut State, inputs: &Vec<bsec_input_t>) -> Vec<bsec_output_t> {
    let mut sensor_outputs = Vec::new();
    let mut n_sensor_outputs: u8 = state.requested_virtual_sensors.len() as u8;

    for _ in 0..state.requested_virtual_sensors.len() {
        sensor_outputs.push(bsec_output_t::default());
    }

    unsafe {
        state.result = bsec_do_steps(
            inputs.as_ptr(),
            inputs.len() as u8,
            sensor_outputs.as_mut_ptr(),
            &mut n_sensor_outputs as *mut u8,
        );
    }

    print_result(state, "Do Steps");

    for i in 0..n_sensor_outputs as usize {
        let output = sensor_outputs[i];
        match output.sensor_id as u32 {
            bsec_virtual_sensor_t::BSEC_OUTPUT_STATIC_IAQ => {
                println!("Static IAQ: {}", output.signal);
            }
            bsec_virtual_sensor_t::BSEC_OUTPUT_STABILIZATION_STATUS => {
                println!("Gas sensor stable: {}", output.signal == 1.0);
            }
            bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_TEMPERATURE => {
                println!("Temperature: {}", output.signal);
            }
            bsec_virtual_sensor_t::BSEC_OUTPUT_SENSOR_HEAT_COMPENSATED_HUMIDITY => {
                println!("Humidity: {}", output.signal);
            }
            bsec_virtual_sensor_t::BSEC_OUTPUT_RAW_PRESSURE => {
                println!("Air Pressure: {}", output.signal);
            }
            bsec_virtual_sensor_t::BSEC_OUTPUT_BREATH_VOC_EQUIVALENT => {
                println!("Breath VOC [ppm]: {}", output.signal);
            }
            _ => {
                println!("{:?}", output);
            }
        }
    }

    sensor_outputs[0..n_sensor_outputs as usize].into()
}

pub fn get_bsec_state() -> Vec<u8> {
    let mut serialized_state: Vec<u8> = vec![0; BSEC_MAX_STATE_BLOB_SIZE as usize];
    let mut work_buffer_state: Vec<u8> = vec![0; BSEC_MAX_WORKBUFFER_SIZE as usize];
    let mut n_serialized_state: u32 = 0;

    unsafe {
        bsec_get_state(
            0,
            serialized_state.as_mut_ptr(),
            BSEC_MAX_STATE_BLOB_SIZE,
            work_buffer_state.as_mut_ptr(),
            BSEC_MAX_WORKBUFFER_SIZE,
            &mut n_serialized_state as *mut u32,
        );
    }

    serialized_state[0..n_serialized_state as usize].into()
}

pub fn set_bsec_state(serialized_state: Vec<u8>) {
    let mut work_buffer_state: Vec<u8> = vec![0; BSEC_MAX_WORKBUFFER_SIZE as usize];

    unsafe {
        bsec_set_state(
            serialized_state.as_ptr(),
            serialized_state.len() as u32,
            work_buffer_state.as_mut_ptr(),
            BSEC_MAX_WORKBUFFER_SIZE,
        );
    }
}

fn print_result(state: &State, op_name: &str) {
    if state.result == 0 {
        println!("BSEC {}: OK", op_name);
    } else {
        println!("BSEC {}: Error {}", op_name, state.result);
    }
}
