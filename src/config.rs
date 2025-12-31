use serde::{Deserialize, Serialize};
use serde_json::Value;
use indexmap::IndexMap;
use std::fs;
use std::path::Path;
use json_comments::StripComments;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WaybarConfig {
    #[serde(rename = "modules-left", default)]
    pub modules_left: Vec<String>,
    #[serde(rename = "modules-center", default)]
    pub modules_center: Vec<String>,
    #[serde(rename = "modules-right", default)]
    pub modules_right: Vec<String>,
    
    #[serde(flatten)]
    pub module_definitions: IndexMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WaybarProfile {
    pub config: WaybarConfig,
    #[serde(default)]
    pub style_vars: IndexMap<String, String>,
    #[serde(default)]
    pub layout_css: String,
}

impl WaybarConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let stripped = StripComments::new(content.as_bytes());
        let config: WaybarConfig = serde_json::from_reader(stripped)?;
        Ok(config)
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let mut cleaned = self.clone();
        cleaned.modules_left.retain(|m| !m.is_empty());
        cleaned.modules_center.retain(|m| !m.is_empty());
        cleaned.modules_right.retain(|m| !m.is_empty());
        
        let json = serde_json::to_string_pretty(&cleaned)?;
        fs::write(path, json)?;
        Ok(())
    }
}

impl WaybarProfile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(&path)?;
        
        // Try to parse as WaybarProfile first
        if let Ok(profile) = serde_json::from_str::<WaybarProfile>(&content) {
            return Ok(profile);
        }
        
        // Fallback: try to parse as WaybarConfig and wrap it
        let config = WaybarConfig::from_file(path)?;
        Ok(WaybarProfile {
            config,
            style_vars: IndexMap::new(),
            layout_css: String::new(),
        })
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
