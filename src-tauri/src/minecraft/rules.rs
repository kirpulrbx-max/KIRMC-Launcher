use serde_json::Value;

use crate::paths;

/// Evalúa las reglas de librerías/argumentos de Mojang. Sin reglas = permitido.
pub fn allowed(rules: Option<&Vec<Value>>) -> bool {
    let Some(rules) = rules else { return true };
    let mut allow = false;
    for rule in rules {
        if matches(rule) {
            allow = rule["action"].as_str() == Some("allow");
        }
    }
    allow
}

fn matches(rule: &Value) -> bool {
    if let Some(os) = rule.get("os") {
        if let Some(name) = os["name"].as_str() {
            if name != paths::os_name() {
                return false;
            }
        }
        if os["arch"].as_str() == Some("x86") && cfg!(target_pointer_width = "64") {
            return false;
        }
    }
    // No activamos features opcionales (demo, resolución custom, quickplay vía reglas)
    if let Some(features) = rule.get("features").and_then(Value::as_object) {
        if features.values().any(|v| v.as_bool() == Some(true)) {
            return false;
        }
    }
    true
}
