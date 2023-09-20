use crate::constants::bill_types;

use super::{disable, enable, init, poll, reset, return_bill, set_security};

pub fn start() {
    let price: u32 = 2;
    let mut total: u32 = 0;

    let mut port = init();

    disable(&mut port);

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
            _ => (),
        }
    }
}
