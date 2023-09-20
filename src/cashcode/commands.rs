use std::{thread, time::Duration};

use serialport::SerialPort;

use crate::{
    constants::{ACK, DISABLE, ENABLE, POLL, RESET, RETURN, SECURITY, STACK},
    tools::format_to_hex,
};

pub fn reset(port: &mut Box<dyn SerialPort>) {
    let mut response: Vec<u8> = vec![0; 6];

    port.write(&RESET).expect("Error with RESET write");
    port.read(response.as_mut_slice())
        .expect("Error with RESET read");

    println!(
        "{:?} -> {:?}",
        format_to_hex(&RESET),
        format_to_hex(&response)
    );
}

pub fn enable(port: &mut Box<dyn SerialPort>) {
    let mut response: Vec<u8> = vec![0; 6];

    port.write(&ENABLE).expect("Error with ENABLE write");
    port.read(response.as_mut_slice())
        .expect("Error with ENABLE read");

    println!(
        "{:?} -> {:?}",
        format_to_hex(&ENABLE),
        format_to_hex(&response)
    );
}

pub fn disable(port: &mut Box<dyn SerialPort>) {
    let mut response: Vec<u8> = vec![0; 6];

    port.write(&DISABLE).expect("Error with DISABLE write");
    port.read(response.as_mut_slice())
        .expect("Error with DISABLE read");

    println!(
        "{:?} -> {:?}",
        format_to_hex(&DISABLE),
        format_to_hex(&response)
    );
}

pub fn set_security(port: &mut Box<dyn SerialPort>) {
    let mut response: Vec<u8> = vec![0; 6];

    port.write(&SECURITY).expect("Error with SECURITY write");
    port.read(response.as_mut_slice())
        .expect("Error with SECURITY read");

    println!(
        "{:?} -> {:?}",
        format_to_hex(&SECURITY),
        format_to_hex(&response)
    );

    thread::sleep(Duration::from_secs(2));
}

pub fn poll(port: &mut Box<dyn SerialPort>) -> [u8; 2] {
    port.write(&POLL).expect("Error with POLL write");

    thread::sleep(Duration::from_millis(10));

    let mut response: Vec<u8> = vec![];

    loop {
        let mut byte: Vec<u8> = vec![0; 1];

        match port.read(byte.as_mut_slice()) {
            Ok(b) if b == 1 => response.push(byte[0]),
            Ok(_) => break,
            Err(error) => {
                print!("\n{} -> ", error);
                break;
            }
        };
    }

    println!(
        "{:?} -> {:?}",
        format_to_hex(&POLL),
        format_to_hex(&response)
    );

    port.write(&ACK).expect("Error with ACK write");

    thread::sleep(Duration::from_secs(1));

    let mut result: [u8; 2] = [0; 2];
    result.copy_from_slice(&response[3..5]);

    result
}

pub fn return_bill(port: &mut Box<dyn SerialPort>) {
    let mut response: Vec<u8> = vec![0; 6];

    port.write(&RETURN).expect("Error with RETURN write");
    port.read(response.as_mut_slice())
        .expect("Error with RETURN read");

    println!(
        "{:?} -> {:?}",
        format_to_hex(&RETURN),
        format_to_hex(&response)
    );
}

pub fn stack(port: &mut Box<dyn SerialPort>) {
    let mut response: Vec<u8> = vec![0; 6];

    port.write(&STACK).expect("Error with STACK write");
    port.read(response.as_mut_slice())
        .expect("Error with STACK read");

    println!(
        "{:?} -> {:?}",
        format_to_hex(&STACK),
        format_to_hex(&response)
    );
}
