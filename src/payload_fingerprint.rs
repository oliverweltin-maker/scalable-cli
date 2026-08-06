use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn input_fingerprint_payload(normalized_input: &Value) -> Value {
    let mut payload = normalized_input.clone();
    if let Some(object) = payload.as_object_mut() {
        object.remove("cursor");
    }
    payload
}

pub(crate) fn checksum_for_payload(payload: &Value) -> String {
    let canonical = canonicalize_json(payload);
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string().as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut sorted = serde_json::Map::with_capacity(map.len());
            for key in keys {
                if let Some(child) = map.get(&key) {
                    sorted.insert(key, canonicalize_json(child));
                }
            }
            Value::Object(sorted)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{checksum_for_payload, input_fingerprint_payload};

    #[test]
    fn input_fingerprint_excludes_cursor_and_canonicalizes_keys() {
        let first = json!({"status": ["CONFIRMED"], "cursor": "page-1", "pageSize": 20});
        let second = json!({"pageSize": 20, "status": ["CONFIRMED"], "cursor": "page-2"});

        assert_eq!(
            checksum_for_payload(&input_fingerprint_payload(&first)),
            checksum_for_payload(&input_fingerprint_payload(&second))
        );
    }
}
