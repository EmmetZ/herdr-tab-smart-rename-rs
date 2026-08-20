use serde_json::Value;

#[test]
fn manifest_uses_rust_binary_without_bun() {
    let manifest = std::fs::read_to_string("herdr-plugin.toml").unwrap();
    assert!(!manifest.contains("bun"));

    let value: toml::Value = toml::from_str(&manifest).unwrap();
    let json = serde_json::to_value(value).unwrap();

    assert_eq!(json["id"], Value::String("tab-smart-rename".into()));
    assert_eq!(
        json["platforms"],
        Value::Array(vec![
            Value::String("linux".into()),
            Value::String("macos".into()),
        ])
    );
    assert_eq!(
        json["build"][0]["command"][0],
        Value::String("cargo".into())
    );
    assert!(
        json["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["id"] == "rename-now")
    );
    assert!(
        json["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["on"] == "pane.agent_status_changed")
    );
}
