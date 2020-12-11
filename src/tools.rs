pub fn empty_string_as_option(value: &str) -> Option<String> {
    match value.len() {
        0 => None,
        _ => Some(value.to_string()),
    }
}
