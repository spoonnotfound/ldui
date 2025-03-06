use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use crate::core::error::LdUiError;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub discourse: DiscourseConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiscourseConfig {
    pub url: String,
    pub api_key: String,
}

impl Config {
    pub fn default() -> Self {
        Config {
            discourse: DiscourseConfig {
                url: "https://linux.do".to_string(),
                api_key: "".to_string(),
            },
        }
    }

    /// 检查配置中是否设置了有效的 API Key
    pub fn has_valid_api_key(&self) -> bool {
        !self.discourse.api_key.is_empty()
    }

    pub fn load() -> color_eyre::Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }
        
        let config_str = fs::read_to_string(&config_path)
            .map_err(|e| LdUiError::Config(format!("无法读取配置文件: {}", e)))?;
            
        let config: Config = toml::from_str(&config_str)
            .map_err(|e| LdUiError::Config(format!("无法解析配置文件: {}", e)))?;
            
        Ok(config)
    }
    
    pub fn save(&self) -> color_eyre::Result<()> {
        let config_path = Self::config_path()?;
        
        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| LdUiError::Config(format!("无法创建配置目录: {}", e)))?;
        }
        
        let config_str = toml::to_string(self)
            .map_err(|e| LdUiError::Config(format!("无法序列化配置: {}", e)))?;
            
        fs::write(&config_path, config_str)
            .map_err(|e| LdUiError::Config(format!("无法写入配置文件: {}", e)))?;
            
        Ok(())
    }
    
    fn config_path() -> color_eyre::Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| LdUiError::Config("无法确定配置目录".to_string()))?;
            
        path.push("ldui");
        path.push("config.toml");
        
        Ok(path)
    }
} 