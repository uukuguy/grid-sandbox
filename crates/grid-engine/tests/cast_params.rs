use grid_engine::tools::cast_params::cast_params;
use serde_json::json;

fn make_schema(properties: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "object",
        "properties": properties
    })
}

#[test]
fn string_to_integer() {
    let params = json!({"count": "42"});
    let schema = make_schema(json!({"count": {"type": "integer"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["count"], json!(42));
    assert_eq!(result.casts_applied.len(), 1);
    assert_eq!(result.casts_applied[0].field, "count");
    assert_eq!(result.casts_applied[0].from_type, "string");
    assert_eq!(result.casts_applied[0].to_type, "integer");
}

#[test]
fn string_to_number() {
    let params = json!({"rate": "3.14"});
    let schema = make_schema(json!({"rate": {"type": "number"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["rate"], json!(3.14));
    assert_eq!(result.casts_applied.len(), 1);
}

#[test]
fn string_to_boolean_true() {
    let params = json!({"verbose": "true"});
    let schema = make_schema(json!({"verbose": {"type": "boolean"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["verbose"], json!(true));
    assert_eq!(result.casts_applied.len(), 1);
}

#[test]
fn string_to_boolean_false() {
    let params = json!({"verbose": "False"});
    let schema = make_schema(json!({"verbose": {"type": "boolean"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["verbose"], json!(false));
    assert_eq!(result.casts_applied.len(), 1);
}

#[test]
fn integer_to_string() {
    let params = json!({"id": 42});
    let schema = make_schema(json!({"id": {"type": "string"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["id"], json!("42"));
    assert_eq!(result.casts_applied.len(), 1);
    assert_eq!(result.casts_applied[0].from_type, "number");
    assert_eq!(result.casts_applied[0].to_type, "string");
}

#[test]
fn boolean_to_string() {
    let params = json!({"flag": true});
    let schema = make_schema(json!({"flag": {"type": "string"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["flag"], json!("true"));
    assert_eq!(result.casts_applied.len(), 1);
    assert_eq!(result.casts_applied[0].from_type, "boolean");
    assert_eq!(result.casts_applied[0].to_type, "string");
}

#[test]
fn already_matching_type_no_cast() {
    let params = json!({"count": 42, "name": "alice", "flag": true});
    let schema = make_schema(json!({
        "count": {"type": "integer"},
        "name": {"type": "string"},
        "flag": {"type": "boolean"}
    }));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params, params);
    assert!(result.casts_applied.is_empty());
}

#[test]
fn field_not_in_schema_left_untouched() {
    let params = json!({"count": "42", "extra": "hello"});
    let schema = make_schema(json!({"count": {"type": "integer"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["count"], json!(42));
    assert_eq!(result.params["extra"], json!("hello"));
    assert_eq!(result.casts_applied.len(), 1);
}

#[test]
fn invalid_cast_not_applied() {
    let params = json!({"count": "abc"});
    let schema = make_schema(json!({"count": {"type": "integer"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["count"], json!("abc"));
    assert!(result.casts_applied.is_empty());
}

#[test]
fn multiple_casts_in_single_call() {
    let params = json!({"count": "10", "rate": "2.5", "verbose": "TRUE", "id": 99});
    let schema = make_schema(json!({
        "count": {"type": "integer"},
        "rate": {"type": "number"},
        "verbose": {"type": "boolean"},
        "id": {"type": "string"}
    }));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["count"], json!(10));
    assert_eq!(result.params["rate"], json!(2.5));
    assert_eq!(result.params["verbose"], json!(true));
    assert_eq!(result.params["id"], json!("99"));
    assert_eq!(result.casts_applied.len(), 4);
}

#[test]
fn empty_params_and_schema() {
    let params = json!({});
    let schema = make_schema(json!({}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params, json!({}));
    assert!(result.casts_applied.is_empty());
}

#[test]
fn non_object_params_returned_as_is() {
    let params = json!("just a string");
    let schema = make_schema(json!({"x": {"type": "integer"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params, json!("just a string"));
    assert!(result.casts_applied.is_empty());
}

#[test]
fn negative_integer_cast() {
    let params = json!({"offset": "-5"});
    let schema = make_schema(json!({"offset": {"type": "integer"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["offset"], json!(-5));
    assert_eq!(result.casts_applied.len(), 1);
}

#[test]
fn negative_number_cast() {
    let params = json!({"temp": "-12.5"});
    let schema = make_schema(json!({"temp": {"type": "number"}}));
    let result = cast_params(&params, &schema);
    assert_eq!(result.params["temp"], json!(-12.5));
    assert_eq!(result.casts_applied.len(), 1);
}
