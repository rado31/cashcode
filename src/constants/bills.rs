pub fn bill_types(bill: u8) -> u32 {
    match bill {
        0 => 1,
        2 => 5,
        3 => 10,
        4 => 20,
        5 => 50,
        6 => 100,
        _ => 0,
    }
}
