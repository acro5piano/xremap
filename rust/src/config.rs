use log::warn;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub windows: Vec<WindowConfig>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WindowConfig {
    pub class_only: Option<Vec<String>>,
    pub class_not: Option<Vec<String>>,
    pub remaps: Vec<Remap>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Remap {
    pub from: String,
    pub to: KeyAction,
}

#[derive(Debug, Clone, Serialize)]
pub enum KeyAction {
    Single(String),
    Multiple(Vec<String>),
}

impl<'de> Deserialize<'de> for WindowConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut map = HashMap::<String, Value>::deserialize(deserializer)?;

        let class_only = map
            .remove("class_only")
            .and_then(|v| serde_yaml::from_value::<Vec<String>>(v).ok());
        let class_not = map
            .remove("class_not")
            .and_then(|v| serde_yaml::from_value::<Vec<String>>(v).ok());

        let remaps_value = map
            .remove("remaps")
            .ok_or_else(|| serde::de::Error::missing_field("remaps"))?;

        let remaps_list =
            serde_yaml::from_value::<Vec<Value>>(remaps_value).map_err(serde::de::Error::custom)?;

        let mut remaps = Vec::new();
        for remap_value in remaps_list {
            if let Value::Mapping(map) = remap_value {
                for (key, value) in map {
                    let from =
                        serde_yaml::from_value::<String>(key).map_err(serde::de::Error::custom)?;

                    let to = match value {
                        Value::String(s) => KeyAction::Single(s),
                        Value::Sequence(seq) => {
                            let strings = seq
                                .into_iter()
                                .map(|v| serde_yaml::from_value::<String>(v))
                                .collect::<Result<Vec<_>, _>>()
                                .map_err(serde::de::Error::custom)?;
                            KeyAction::Multiple(strings)
                        }
                        _ => return Err(serde::de::Error::custom("Invalid 'to' value")),
                    };

                    remaps.push(Remap { from, to });
                }
            }
        }

        Ok(WindowConfig {
            class_only,
            class_not,
            remaps,
        })
    }
}

impl Config {
    pub fn from_yaml(content: &str) -> anyhow::Result<Self> {
        let config: Config = serde_yaml::from_str(content)?;
        Ok(config)
    }

    pub fn remaps_for_window(&self, window_class: Option<&str>) -> Vec<Remap> {
        let mut remaps = Vec::new();

        for window_config in &self.windows {
            if self.matches_window(window_config, window_class) {
                for remap in &window_config.remaps {
                    remaps.push(remap.clone());
                }
            }
        }

        remaps
    }

    fn matches_window(&self, config: &WindowConfig, window_class: Option<&str>) -> bool {
        // If both class_only and class_not are None, this rule applies to all windows
        if config.class_only.is_none() && config.class_not.is_none() {
            return true;
        }

        let class = match window_class {
            Some(c) => c.to_lowercase(),
            None => {
                // If no window class detected:
                // - class_not rules apply (since we can't exclude what we don't know)
                // - class_only rules don't apply (since we can't match what we don't know)
                // But let's be more permissive for better UX
                warn!("No window class detected - this may prevent class_only rules from working");
                if config.class_not.is_some() {
                    return true; // Apply class_not rules when no class detected
                }
                // For class_only, let's try a more permissive approach
                return false; // Don't apply class_only rules when no class detected
            }
        };

        if let Some(ref class_only) = config.class_only {
            return class_only.iter().any(|c| class.contains(&c.to_lowercase()));
        }

        if let Some(ref class_not) = config.class_not {
            return !class_not.iter().any(|c| class.contains(&c.to_lowercase()));
        }

        true
    }
}
