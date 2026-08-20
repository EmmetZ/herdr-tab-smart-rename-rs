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
    assert_eq!(json["build"][0]["command"][0], Value::String("bash".into()));
    assert_eq!(
        json["build"][0]["command"][1],
        Value::String("herdr/install.sh".into())
    );
    assert!(
        json["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["id"] == "rename-now"
                && action["command"][0] == Value::String("sh".into())
                && action["command"][1] == Value::String("-c".into())
                && action["command"][2]
                    == Value::String(
                        "exec \"$HERDR_PLUGIN_ROOT/bin/herdr-tab-smart-rename-rs\" rename-now"
                            .into()
                    ))
    );
    assert!(
        json["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["on"] == "pane.agent_status_changed")
    );
}
