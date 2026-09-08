//! Regression tests for Gradio 6.x parameter label i18n maps (issue #10).
//!
//! Gradio 5.x emits `label` as a plain string; 6.x wraps it in a
//! translation-metadata object (`{ key, _type }`). Both shapes must
//! deserialize into `ApiData.label: Option<String>`.

use gradio::structs::ApiData;

const GRADIO_5_LABEL: &str = r#"{
    "label": "Input text",
    "parameter_name": "text",
    "component": "Textbox",
    "type": { "type": "string" },
    "python_type": { "type": "str", "description": "" }
}"#;

const GRADIO_6_LABEL: &str = r#"{
    "label": {
        "key": "show_prompt_text_label",
        "_type": "translation_metadata"
    },
    "parameter_name": "checked",
    "component": "Checkbox",
    "type": { "type": "boolean" },
    "python_type": { "type": "bool", "description": "" }
}"#;

#[test]
fn label_accepts_plain_string_gradio_5() {
    let p: ApiData = serde_json::from_str(GRADIO_5_LABEL).unwrap();
    assert_eq!(p.label.as_deref(), Some("Input text"));
    assert_eq!(p.parameter_name.as_deref(), Some("text"));
}

#[test]
fn label_accepts_i18n_map_gradio_6() {
    let p: ApiData = serde_json::from_str(GRADIO_6_LABEL).unwrap();
    // 6.x i18n map is normalized to its inner key
    assert_eq!(p.label.as_deref(), Some("show_prompt_text_label"));
    assert_eq!(p.parameter_name.as_deref(), Some("checked"));
}

#[test]
fn label_can_be_absent() {
    let json = r#"{
        "parameter_name": "seed",
        "component": "Number",
        "type": { "type": "number" },
        "python_type": { "type": "float", "description": "" }
    }"#;
    let p: ApiData = serde_json::from_str(json).unwrap();
    assert!(p.label.is_none());
}

#[test]
fn label_can_be_null() {
    let mut json: serde_json::Value = serde_json::from_str(GRADIO_5_LABEL).unwrap();
    json["label"] = serde_json::Value::Null;
    let p: ApiData = serde_json::from_value(json).unwrap();
    assert!(p.label.is_none());
}

#[test]
fn label_rejects_invalid_shapes() {
    use serde_json::json;

    for label in [
        json!(42),
        json!(true),
        json!([]),
        json!({}),
        json!({"key": "Input text"}),
        json!({"_type": "translation_metadata"}),
        json!({"_type": "translation_metadata", "key": null}),
        json!({"_type": "translation_metadata", "key": 42}),
        json!({"_type": "other", "key": "Input text"}),
    ] {
        let mut json: serde_json::Value = serde_json::from_str(GRADIO_5_LABEL).unwrap();
        json["label"] = label.clone();
        assert!(
            serde_json::from_value::<ApiData>(json).is_err(),
            "accepted invalid label: {label}"
        );
    }
}

#[test]
fn endpoint_labels_are_normalized_for_parameters_and_returns() {
    use gradio::structs::ApiInfo;
    use serde_json::json;

    let translated: serde_json::Value = serde_json::from_str(GRADIO_6_LABEL).unwrap();
    let api: ApiInfo = serde_json::from_value(json!({
        "named_endpoints": {
            "/predict": {
                "parameters": [translated.clone()],
                "returns": [translated]
            }
        }
    }))
    .unwrap();
    let endpoint = &api.named_endpoints["/predict"];
    assert_eq!(
        endpoint.parameters[0].label.as_deref(),
        Some("show_prompt_text_label")
    );
    assert_eq!(
        endpoint.returns[0].label.as_deref(),
        Some("show_prompt_text_label")
    );

    let serialized = serde_json::to_value(&api).unwrap();
    assert_eq!(
        serialized["named_endpoints"]["/predict"]["returns"][0]["label"],
        "show_prompt_text_label"
    );
    let round_trip: ApiInfo = serde_json::from_value(serialized).unwrap();
    assert_eq!(
        round_trip.named_endpoints["/predict"].returns[0].label,
        endpoint.returns[0].label
    );
}
