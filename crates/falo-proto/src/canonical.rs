//! JSON canonico (RFC 8785) per firme deterministiche.

use serde::Serialize;

/// Serializza `value` in JSON canonico: chiavi ordinate, numeri normalizzati.
pub fn to_canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, serde_json::Error> {
    serde_json_canonicalizer::to_vec(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_are_sorted_recursively() {
        let v = json!({ "b": 1, "a": { "z": true, "c": [3, 1] } });
        let out = String::from_utf8(to_canonical_json(&v).unwrap()).unwrap();
        assert_eq!(out, r#"{"a":{"c":[3,1],"z":true},"b":1}"#);
    }
}
