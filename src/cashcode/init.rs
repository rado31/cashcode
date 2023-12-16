use std::time::Duration;

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

pub fn init() -> Box<dyn SerialPort> {
    serialport::new("/dev/ttyS0", 9600)
        .data_bits(DataBits::Eight)
        .flow_control(FlowControl::None)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .timeout(Duration::from_millis(100))
        .open()
        .unwrap()
}
