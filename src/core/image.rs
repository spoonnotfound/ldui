use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::Mutex;
use anyhow::Result;
use scraper::{Html, Selector};
use tracing::{debug, warn};

/// 图片缓存，用于存储已下载的图片
#[derive(Debug, Clone)]
pub struct ImageCache {
    cache: Arc<Mutex<HashMap<String, PathBuf>>>,
    cache_dir: PathBuf,
}

impl ImageCache {
    /// 创建新的图片缓存
    pub fn new(cache_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&cache_dir).unwrap_or_else(|_| {
            warn!("无法创建图片缓存目录：{:?}", cache_dir);
        });
        
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            cache_dir,
        }
    }
    
    /// 获取图片缓存路径
    pub async fn get_cached_path(&self, url: &str) -> Option<PathBuf> {
        let cache = self.cache.lock().await;
        cache.get(url).cloned()
    }
    
    /// 添加图片到缓存
    pub async fn add_to_cache(&self, url: &str, image_data: &[u8]) -> Result<PathBuf> {
        // 计算文件名（使用URL的哈希）
        let url_hash = format!("{:x}", md5::compute(url.as_bytes()));
        let ext = url.split('.').last().unwrap_or("jpg");
        let filename = format!("{}.{}", url_hash, ext);
        let file_path = self.cache_dir.join(&filename);
        
        // 保存图片数据到文件
        tokio::fs::write(&file_path, image_data).await?;
        
        // 更新缓存
        let mut cache = self.cache.lock().await;
        cache.insert(url.to_string(), file_path.clone());
        
        Ok(file_path)
    }
}

/// 从HTML中提取图片URL
pub fn extract_image_urls(html_content: &str) -> Vec<String> {
    let document = Html::parse_document(html_content);
    
    // 优化：使用更具体的选择器，只选择需要的图片元素
    // 例如，避免选择小图标或头像等
    let selector = Selector::parse("img:not(.avatar):not(.icon)").unwrap_or_else(|_| {
        // 如果选择器无效，回退到基本选择器
        Selector::parse("img").unwrap()
    });
    
    let mut urls = Vec::new();
    
    for element in document.select(&selector) {
        if let Some(src) = element.value().attr("src") {
            // 跳过非图片URL（如data:URL)
            if !src.starts_with("data:") {
                urls.push(src.to_string());
            }
        }
    }
    
    urls
}

/// 异步下载图片
pub async fn download_image(url: &str) -> Result<Vec<u8>> {
    debug!("下载图片: {}", url);
    
    // 发送HTTP请求获取图片
    let response = reqwest::get(url).await?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("下载图片失败: HTTP {}", response.status()));
    }
    
    // 获取图片数据
    let image_data = response.bytes().await?;
    Ok(image_data.to_vec())
}
