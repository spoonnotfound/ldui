use thiserror::Error;

#[derive(Error, Debug)]
pub enum LdUiError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("API错误: {0}")]
    Api(String),
    
    #[error("HTTP请求错误: {0}")]
    Request(#[from] reqwest::Error),
    
    #[error("解析错误: {0}")]
    Parse(String),
    
    #[error("配置错误: {0}")]
    Config(String),
    
    #[error("未经授权")]
    Unauthorized,
}