use serde::{Deserialize, Serialize};
use serde_json::Value;
use reqwest::{Client, header};
use chrono::{DateTime, Utc};
use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, debug, error};

use crate::core::config::DiscourseConfig;
use crate::core::error::LdUiError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Topic {
    pub id: u64,
    pub title: String,
    pub posts_count: u64,
    pub views: u64,
    pub created_at: DateTime<Utc>,
    pub last_posted_at: Option<DateTime<Utc>>,
    pub posters: Vec<Poster>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Poster {
    pub user_id: i64,
    pub primary_group_id: Option<u64>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Post {
    pub id: u64,
    pub topic_id: u64,
    pub user_id: u64,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub cooked: String,
    pub posts_count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Category {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub topic_count: u64,
    pub post_count: u64,
    pub description: Option<String>,
    pub color: String,
    pub text_color: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub name: Option<String>,
    pub avatar_template: String,
    pub trust_level: u64,
}

#[async_trait]
pub trait DiscourseClient {
    async fn get_latest_topics(&self, page: u32) -> Result<Vec<Topic>>;
    #[allow(unused)]
    async fn get_topic(&self, id: u64) -> Result<Topic>;
    async fn get_topic_posts(&self, topic_id: u64, page: u32) -> Result<Vec<Post>>;
    async fn get_categories(&self) -> Result<Vec<Category>>;
    async fn get_user(&self, username: &str) -> Result<User>;
    async fn create_post(&self, topic_id: u64, content: &str) -> Result<Post>;
}

pub struct ApiClient {
    config: DiscourseConfig,
    client: Client,
}

impl ApiClient {
    pub fn new(config: DiscourseConfig) -> Self {
        let mut headers = header::HeaderMap::new();
        
        if !config.api_key.is_empty() {
            headers.insert(
                "Api-Userkey",
                header::HeaderValue::from_str(&config.api_key).unwrap(),
            );
            headers.insert(
                "Api-Username",
                header::HeaderValue::from_str("ldui").unwrap(),
            );
        }

        debug!("headers: {:?}", headers);
        
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
            
        Self { config, client }
    }
    
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.url, path)
    }
}

#[async_trait]
impl DiscourseClient for ApiClient {
    async fn get_latest_topics(&self, page: u32) -> Result<Vec<Topic>> {
        info!("开始获取最新主题列表, 页码: {}", page);
        let url = self.url(&format!("/latest.json?page={}", page-1));
        debug!("请求URL: {}", url);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("请求最新主题失败: {}", e);
                LdUiError::Request(e)
            })?;
            
        if !response.status().is_success() {
            let err_msg = format!("获取主题失败，状态码: {}", response.status());
            error!("{}", err_msg);
            return Err(LdUiError::Api(err_msg).into());
        }
        debug!("获取最新主题成功，状态码: {}", response.status());
        
        let json: Value = response.json().await
            .map_err(|e| {
                error!("解析最新主题响应失败: {}", e);
                LdUiError::Parse(format!("解析响应失败: {}", e))
            })?;

        debug!("响应数据: {}", json);
            
        let topics = json["topic_list"]["topics"]
            .as_array()
            .ok_or_else(|| {
                let err_msg = "无法解析主题列表".to_string();
                error!("{}", err_msg);
                LdUiError::Parse(err_msg)
            })?
            .to_owned();
            
        let topics: Vec<Topic> = serde_json::from_value(Value::Array(topics))
            .map_err(|e| {
                error!("解析主题数据失败: {}", e);
                LdUiError::Parse(format!("解析主题失败: {}", e))
            })?;
            
        info!("成功获取最新主题，共 {} 条", topics.len());
        Ok(topics)
    }
    
    #[allow(unused)]
    async fn get_topic(&self, id: u64) -> Result<Topic> {
        info!("开始获取主题详情, ID: {}", id);
        let url = self.url(&format!("/t/{}.json", id));
        debug!("请求URL: {}", url);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("请求主题详情失败: {}", e);
                LdUiError::Request(e)
            })?;
            
        if !response.status().is_success() {
            let err_msg = format!("获取主题详情失败，状态码: {}", response.status());
            error!("{}", err_msg);
            return Err(LdUiError::Api(err_msg).into());
        }
        debug!("获取主题详情成功，状态码: {}", response.status());
        
        let topic: Topic = response.json().await
            .map_err(|e| {
                error!("解析主题详情失败: {}", e);
                LdUiError::Parse(format!("解析主题失败: {}", e))
            })?;
            
        info!("成功获取主题详情, 标题: {}", topic.title);
        Ok(topic)
    }
    
    async fn get_topic_posts(&self, topic_id: u64, page: u32) -> Result<Vec<Post>> {
        info!("开始获取主题帖子, 主题ID: {}, 页码: {}", topic_id, page);
        let url = self.url(&format!("/t/topic/{}.json?page={}", topic_id, page));
        debug!("请求URL: {}", url);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("请求主题帖子失败: {}", e);
                LdUiError::Request(e)
            })?;
            
        if !response.status().is_success() {
            let err_msg = format!("获取帖子失败，状态码: {}", response.status());
            error!("{}", err_msg);
            return Err(LdUiError::Api(err_msg).into());
        }
        debug!("获取主题帖子成功，状态码: {}", response.status());
        
        let json: Value = response.json().await
            .map_err(|e| {
                error!("解析主题帖子响应失败: {}", e);
                LdUiError::Parse(format!("解析响应失败: {}", e))
            })?;
            
        let posts = json["post_stream"]["posts"]
            .as_array()
            .ok_or_else(|| {
                let err_msg = "无法解析帖子列表".to_string();
                error!("{}", err_msg);
                LdUiError::Parse(err_msg)
            })?
            .to_owned();
            
        let posts: Vec<Post> = serde_json::from_value(Value::Array(posts))
            .map_err(|e| {
                error!("解析帖子数据失败: {}", e);
                LdUiError::Parse(format!("解析帖子失败: {}", e))
            })?;
            
        info!("成功获取主题帖子，共 {} 条", posts.len());
        Ok(posts)
    }
    
    async fn get_categories(&self) -> Result<Vec<Category>> {
        info!("开始获取分类列表");
        let url = self.url("/categories.json");
        debug!("请求URL: {}", url);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("请求分类列表失败: {}", e);
                LdUiError::Request(e)
            })?;
            
        if !response.status().is_success() {
            let err_msg = format!("获取分类失败，状态码: {}", response.status());
            error!("{}", err_msg);
            return Err(LdUiError::Api(err_msg).into());
        }
        debug!("获取分类列表成功，状态码: {}", response.status());
        
        let json: Value = response.json().await
            .map_err(|e| {
                error!("解析分类列表响应失败: {}", e);
                LdUiError::Parse(format!("解析响应失败: {}", e))
            })?;
            
        let categories = json["category_list"]["categories"]
            .as_array()
            .ok_or_else(|| {
                let err_msg = "无法解析分类列表".to_string();
                error!("{}", err_msg);
                LdUiError::Parse(err_msg)
            })?
            .to_owned();
            
        let categories: Vec<Category> = serde_json::from_value(Value::Array(categories))
            .map_err(|e| {
                error!("解析分类数据失败: {}", e);
                LdUiError::Parse(format!("解析分类失败: {}", e))
            })?;
            
        info!("成功获取分类列表，共 {} 个分类", categories.len());
        Ok(categories)
    }
    
    async fn get_user(&self, username: &str) -> Result<User> {
        info!("开始获取用户信息, 用户名: {}", username);
        let url = self.url(&format!("/users/{}.json", username));
        debug!("请求URL: {}", url);
        
        let response = self.client.get(&url)
            .send()
            .await
            .map_err(|e| {
                error!("请求用户信息失败: {}", e);
                LdUiError::Request(e)
            })?;
            
        if !response.status().is_success() {
            let err_msg = format!("获取用户失败，状态码: {}", response.status());
            error!("{}", err_msg);
            return Err(LdUiError::Api(err_msg).into());
        }
        debug!("获取用户信息成功，状态码: {}", response.status());
        
        let json: Value = response.json().await
            .map_err(|e| {
                error!("解析用户信息响应失败: {}", e);
                LdUiError::Parse(format!("解析响应失败: {}", e))
            })?;
            
        let user = json["user"].clone();
        
        let user: User = serde_json::from_value(user)
            .map_err(|e| {
                error!("解析用户数据失败: {}", e);
                LdUiError::Parse(format!("解析用户失败: {}", e))
            })?;
            
        info!("成功获取用户信息, 用户名: {}, ID: {}", user.username, user.id);
        Ok(user)
    }
    
    async fn create_post(&self, topic_id: u64, content: &str) -> Result<Post> {
        info!("开始创建帖子, 主题ID: {}", topic_id);
        
        if self.config.api_key.is_empty() {
            error!("创建帖子失败: API密钥为空");
            return Err(LdUiError::Unauthorized.into());
        }
        
        let url = self.url("/t/posts.json");
        debug!("请求URL: {}", url);
        
        let content_preview = if content.len() > 50 {
            format!("{}...", &content[..47])
        } else {
            content.to_string()
        };
        debug!("发布内容预览: {}", content_preview);
        
        let params = [
            ("topic_id", topic_id.to_string()),
            ("raw", content.to_string()),
        ];
        
        let response = self.client.post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| {
                error!("发送创建帖子请求失败: {}", e);
                LdUiError::Request(e)
            })?;
            
        if !response.status().is_success() {
            let err_msg = format!("创建帖子失败，状态码: {}", response.status());
            error!("{}", err_msg);
            return Err(LdUiError::Api(err_msg).into());
        }
        debug!("创建帖子请求成功，状态码: {}", response.status());
        
        let post: Post = response.json().await
            .map_err(|e| {
                error!("解析创建的帖子数据失败: {}", e);
                LdUiError::Parse(format!("解析帖子失败: {}", e))
            })?;
            
        info!("成功创建帖子, 帖子ID: {}", post.id);
        Ok(post)
    }
} 