use serde_json::Value;

/// Record of a single type cast applied to a parameter.
#[derive(Debug, Clone)]
pub struct CastApplied {
    pub field: String,
    pub from_type: String,
    pub to_type: String,
}

/// Result of automatic parameter type casting.
#[derive(Debug, Clone)]
pub struct CastResult {
    pub params: Value,
    pub casts_applied: Vec<CastApplied>,
}

/// Automatically cast LLM-returned parameters to match the expected JSON Schema types.
///
/// LLMs frequently return values with incorrect JSON types:
/// - String `"42"` when the schema expects an integer `42`
/// - String `"true"` when the schema expects a boolean `true`
/// - String `"3.14"` when the schema expects a number `3.14`
/// - Integer `42` when the schema expects a string `"42"`
///
/// This function inspects each top-level property in `params` against the
/// corresponding `"type"` declaration in `schema.properties` and attempts
/// a lossless conversion. Fields that already match, lack a schema entry,
/// or cannot be converted are left untouched.
pub fn cast_params(params: &Value, schema: &Value) -> CastResult {
    let mut result = params.clone();
    let mut casts = Vec::new();

    if let (Value::Object(params_obj), Some(Value::Object(props))) =
        (&mut result, schema.get("properties"))
    {
        for (key, prop_schema) in props {
            if let Some(param_value) = params_obj.get_mut(key) {
                if let Some(expected_type) = prop_schema.get("type").and_then(|t| t.as_str()) {
                    if let Some(cast) = try_cast(key, param_value, expected_type) {
                        casts.push(cast);
                    }
                }
            }
        }
    }

    CastResult {
        params: result,
        casts_applied: casts,
    }
}

/// Attempt to cast `value` in-place to match `expected_type`.
/// Returns `Some(CastApplied)` if a conversion was performed, `None` otherwise.
fn try_cast(field: &str, value: &mut Value, expected_type: &str) -> Option<CastApplied> {
    match (value.clone(), expected_type) {
        // String "42" -> integer 42
        (Value::String(s), "integer") => {
            let n: i64 = s.parse().ok()?;
            *value = Value::Number(n.into());
            Some(CastApplied {
                field: field.to_string(),
                from_type: "string".into(),
                to_type: "integer".into(),
            })
        }
        // String "3.14" -> number 3.14
        (Value::String(s), "number") => {
            let n: f64 = s.parse().ok()?;
            *value = serde_json::Number::from_f64(n).map(Value::Number)?;
            Some(CastApplied {
                field: field.to_string(),
                from_type: "string".into(),
                to_type: "number".into(),
            })
        }
        // String "true"/"false" -> boolean
        (Value::String(s), "boolean") => match s.to_lowercase().as_str() {
            "true" => {
                *value = Value::Bool(true);
                Some(CastApplied {
                    field: field.to_string(),
                    from_type: "string".into(),
                    to_type: "boolean".into(),
                })
            }
            "false" => {
                *value = Value::Bool(false);
                Some(CastApplied {
                    field: field.to_string(),
                    from_type: "string".into(),
                    to_type: "boolean".into(),
                })
            }
            _ => None,
        },
        // Number -> string
        (Value::Number(n), "string") => {
            *value = Value::String(n.to_string());
            Some(CastApplied {
                field: field.to_string(),
                from_type: "number".into(),
                to_type: "string".into(),
            })
        }
        // Bool -> string
        (Value::Bool(b), "string") => {
            *value = Value::String(b.to_string());
            Some(CastApplied {
                field: field.to_string(),
                from_type: "boolean".into(),
                to_type: "string".into(),
            })
        }
        _ => None,
    }
}
