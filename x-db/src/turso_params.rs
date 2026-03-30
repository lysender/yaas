use turso::Value;

pub fn new_query_params() -> Vec<(String, Value)> {
    Vec::new()
}

pub fn text_param(key: &str, value: String) -> (String, Value) {
    (key.to_string(), Value::Text(value))
}

pub fn integer_param(key: &str, value: i64) -> (String, Value) {
    (key.to_string(), Value::Integer(value))
}
