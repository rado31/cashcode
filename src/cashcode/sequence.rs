use crate::{constants::bill_types, tools::format_to_hex};

use super::{disable, enable, init, poll, reset, return_bill, set_security};

pub fn start() {
    let price: f32 = 50.0;
    let mut total: f32 = 0.0;

    let mut port = init();

    reset(&mut port);
    enable(&mut port);

    loop {
        let response = poll(&mut port);

        match response[0] {
            // 0x13
            19 => {
                set_security(&mut port);
            }
            // 0x80
            128 => {
                return_bill(&mut port);

                let amount = bill_types(response[1]);

                total += amount;

                println!("Total: {}", total);

                if total < price {
                    continue;
                } else {
                    disable(&mut port);
                    break;
                }
            }
            // 1C - rejection
            28 => match response[1] {
                // verification
                102 => println!("VERIFICATION ERROR"),
                // others
                _ => println!("{:?}", format_to_hex(&response)),
            },
            _ => println!("OTHER RES: {:?}", format_to_hex(&response)),
        }
    }
}
