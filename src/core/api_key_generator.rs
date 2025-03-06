use std::io;
use std::error::Error;
use webbrowser;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use rand::{thread_rng, Rng};
use rand_core::OsRng;
use rsa::{RsaPrivateKey, RsaPublicKey, pkcs8::EncodePublicKey};
use rsa::pkcs8::LineEnding;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use urlencoding::encode;

use crate::core::config::{Config, DiscourseConfig};


#[derive(Debug, Deserialize, Serialize)]
pub struct UserApiKeyPayload {
    pub key: String,
    pub nonce: String,
    pub push: bool,
    pub api: i32,
}

#[derive(Debug)]
pub struct UserApiKeyRequestResult {
    pub payload: UserApiKeyPayload,
}

// 生成随机字符串作为 nonce
fn generate_nonce() -> String {
    let mut rng = thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
    BASE64.encode(&random_bytes)
}

pub fn generate_user_api_key(
    site_url_base: &str,
    application_name: &str
) -> Result<UserApiKeyRequestResult, Box<dyn Error>> {
    // 生成 RSA 密钥对
    let mut rng = OsRng;
    let private_key = RsaPrivateKey::new(&mut rng, 2048)?;
    let public_key = RsaPublicKey::from(&private_key);
    
    // 转换为 PEM 格式
    let public_key_pem = public_key.to_public_key_pem(LineEnding::LF)?;
    
    // 生成随机 client_id (如果未提供)
    let client_id_to_use = Uuid::new_v4().to_string();
    
    // 生成随机 nonce
    let nonce = generate_nonce();
    
    // 构建请求 URL
    let params = vec![
        format!("application_name={}", encode(application_name)),
        format!("client_id={}", encode(&client_id_to_use)),
        format!("scopes=read"),
        format!("public_key={}", encode(&public_key_pem)),
        format!("nonce={}", encode(&nonce)),
    ];
    
    let url = format!("{}/user-api-key/new?{}", site_url_base, params.join("&"));
    
    // 打开浏览器
    println!("正在打开浏览器获取 API 密钥...");
    if let Err(e) = webbrowser::open(&url) {
        println!("无法自动打开浏览器: {}", e);
        println!("请手动打开以下链接:");
        println!("{}", url);
    }
    
    // 接收用户输入的响应 payload
    println!();
    println!("请在浏览器中完成授权后，将响应的 payload 粘贴到这里:");
    
    let mut enc_payload = String::new();
    io::stdin().read_line(&mut enc_payload)?;
    // 移除所有空格、换行符、制表符和其他空白字符
    enc_payload = enc_payload.chars().filter(|c| !c.is_whitespace()).collect();
    
    // 解密响应
    let enc_payload_bytes = BASE64.decode(enc_payload)?;
    let dec_payload_bytes = private_key.decrypt(rsa::Pkcs1v15Encrypt, &enc_payload_bytes)?;
    let dec_payload_str = String::from_utf8(dec_payload_bytes)?;
    
    // 解析 JSON
    let payload: UserApiKeyPayload = serde_json::from_str(&dec_payload_str)?;
    
    // 验证 nonce
    if payload.nonce != nonce {
        return Err("Nonce 不匹配，可能存在安全风险".into());
    }
    
    Ok(UserApiKeyRequestResult {
        payload,
    })
}

pub fn update_config_with_api_key(
    api_key: &str,
    site_url: &str,
) -> Result<(), Box<dyn Error>> {
    // 加载当前配置
    let mut config = Config::load().map_err(|e| format!("加载配置失败: {}", e))?;
    
    // 更新配置
    config.discourse = DiscourseConfig {
        url: site_url.to_string(),
        api_key: api_key.to_string(),
    };
    
    // 保存配置
    config.save().map_err(|e| format!("保存配置失败: {}", e))?;
    
    Ok(())
}

pub fn run_key_generator() -> Result<(), Box<dyn Error>> {
    println!("=== Linux Do API 密钥生成器 ===");
    println!("该工具将帮助您生成用于访问 Linux Do 论坛的 API 密钥");
    println!();
    
    let mut url = String::new();
    println!("请输入 Linux Do 论坛 URL (默认: https://linux.do):");
    io::stdin().read_line(&mut url)?;
    url = url.trim().to_string();
    if url.is_empty() {
        url = "https://linux.do".to_string();
    }
    
    // 确保 URL 没有结尾的斜杠
    if url.ends_with('/') {
        url.pop();
    }
    
    let mut app_name = String::new();
    println!("请输入应用名称 (用于在 Linux Do 上显示):");
    io::stdin().read_line(&mut app_name)?;
    app_name = app_name.trim().to_string();
    if app_name.is_empty() {
        app_name = "Linux Do 终端客户端".to_string();
    }
    
    println!("开始生成 API 密钥...");
    let result = generate_user_api_key(url.as_str(), app_name.as_str())?;
    
    println!("API 密钥生成成功!");
    println!("API 密钥: {}", result.payload.key);
    
    println!("是否要将此 API 密钥保存到配置文件中? (y/n)");
    let mut save_choice = String::new();
    io::stdin().read_line(&mut save_choice)?;
    
    if save_choice.trim().to_lowercase() == "y" {
        update_config_with_api_key(&result.payload.key, url.as_str())?;
        println!("配置已更新!");
    }
    
    Ok(())
} 