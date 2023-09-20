pub fn format_to_hex(buf: &[u8]) -> Vec<String> {
    buf.iter().map(|value| format!("{:X}", value)).collect()
}
