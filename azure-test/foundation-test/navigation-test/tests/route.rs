#[cfg(test)]
mod tests {
    use azure_foundation::navigation::models::route::*;

    #[test]
    fn encode_then_decode_round_trips() {
        let route = Route::new("/settings", "tab=audio");
        let decoded = Route::decode(&route.encode());
        assert_eq!(decoded.path, "/settings");
        assert_eq!(decoded.payload, "tab=audio");
    }

    #[test]
    fn decoding_a_raw_string_without_payload_keeps_it_as_the_path() {
        let decoded = Route::decode("/home");
        assert_eq!(decoded.path, "/home");
        assert_eq!(decoded.payload, "");
    }
}
