pub mod bme {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    pub struct State {
        device: bme68x_dev,
        result: i8
    }

    fn init(state: &mut State) {
        println!("module loaded");
        let reg_addr: u8 = 0x75;
        let mut data_array = vec![0u8; 5];

        unsafe {
            state.result = bme68x_get_regs(
                reg_addr,
                data_array.as_mut_ptr(),
                0,
                &mut state.device as *mut bme68x_dev,
            );
        }
    }

    fn print_result(state: State, op_name: &str) {
        if state.result == 0 {
            println!("BSEC {}: OK", op_name);
        } else {
            println!("BSEC {}: Error {}", op_name, state.result);
        }
    }
}
