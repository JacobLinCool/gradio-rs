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
