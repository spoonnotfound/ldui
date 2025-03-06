use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use crossterm::event::{KeyEvent, KeyCode};
use std::path::PathBuf;

use crate::core::config::Config;
use crate::api::{DiscourseClient, ApiClient, Topic, Post, Category, User};
use crate::core::image::ImageCache;
use tracing::warn;

pub type AppResult<T> = std::result::Result<T, anyhow::Error>;

#[derive(Debug, Clone, PartialEq)]
pub enum AppTab {
    Home,
    Topics,
    Categories,
    Topic(u64),
    User(String),
    Settings,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoadingState {
    NotLoading,
    Loading,
    Error(String),
}

pub struct App {
    pub config: Config,
    pub client: Arc<dyn DiscourseClient + Send + Sync>,
    pub current_tab: AppTab,
    pub topics: Vec<Topic>,
    pub categories: Vec<Category>,
    pub posts: HashMap<u64, Vec<Post>>,
    pub users: HashMap<String, User>,
    pub selected_index: usize,
    pub page: u32,
    pub loading_state: LoadingState,
    pub show_help: bool,
    pub should_quit: bool,
    pub input: String,
    pub input_mode: bool,
    pub image_cache: ImageCache,
    pub image_paths: Arc<Mutex<HashMap<String, PathBuf>>>,
    pub selected_image_button: Option<usize>,
    pub showing_image: bool,
    pub current_image_url: Option<String>,
    pub need_redraw: bool,
    pub viewing_full_post: bool,
    pub post_scroll: u16,
}

impl App {
    pub fn new(config: Config) -> Self {
        // 创建客户端
        let client = Arc::new(ApiClient::new(config.discourse.clone()));
        
        // 创建图片缓存目录
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("./.cache"))
            .join("ldui/images");
        
        Self {
            config,
            client,
            current_tab: AppTab::Home,
            topics: Vec::new(),
            categories: Vec::new(),
            posts: HashMap::new(),
            users: HashMap::new(),
            selected_index: 0,
            page: 1,
            loading_state: LoadingState::NotLoading,
            show_help: false,
            should_quit: false,
            input: String::new(),
            input_mode: false,
            image_cache: ImageCache::new(cache_dir),
            image_paths: Arc::new(Mutex::new(HashMap::new())),
            selected_image_button: None,
            showing_image: false,
            current_image_url: None,
            need_redraw: false,
            viewing_full_post: false,
            post_scroll: 0,
        }
    }
    
    pub async fn init(&mut self) -> AppResult<()> {
        self.load_topics().await?;
        self.load_categories().await?;
        Ok(())
    }
    
    pub async fn tick(&mut self) -> AppResult<()> {
        // 刷新数据
        if !matches!(self.loading_state, LoadingState::Loading) {
            match self.current_tab.clone() {
                AppTab::Topics => {
                    self.load_topics().await?;
                }
                AppTab::Categories => {
                    self.load_categories().await?;
                }
                AppTab::Topic(id) => {
                    self.load_topic_posts(id).await?;
                }
                AppTab::User(username) => {
                    self.load_user(&username).await?;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> AppResult<()> {
        // 如果正在显示图片，任何按键都会关闭图片显示
        if self.showing_image {
            match key.code {
                KeyCode::Enter | KeyCode::Esc | KeyCode::Char('o') => {
                    self.showing_image = false;
                    self.current_image_url = None;
                    return Ok(());
                }
                _ => return Ok(()), // 忽略其他按键
            }
        }
        
        // 如果正在查看完整帖子
        if self.viewing_full_post {
            match key.code {
                KeyCode::Esc => {
                    // 退出完整帖子查看模式
                    self.viewing_full_post = false;
                    self.post_scroll = 0;
                    return Ok(());
                }
                KeyCode::Char('o') => {
                    // 'o'键用于查看图片
                    if let Some(button_index) = self.selected_image_button {
                        if let Some(posts) = self.posts.get(&self.get_current_topic_id()) {
                            if self.selected_index < posts.len() {
                                let post = &posts[self.selected_index];
                                let image_urls = crate::core::image::extract_image_urls(&post.cooked);
                                
                                // 创建可用图片映射
                                let mut available_images = Vec::new();
                                for (i, url) in image_urls.iter().enumerate() {
                                    if self.image_paths.lock().unwrap().get::<str>(url).is_some() {
                                        available_images.push((i, url.clone()));
                                    }
                                }
                                
                                if button_index < available_images.len() {
                                    // 获取真实的URL
                                    let (_, url) = &available_images[button_index];
                                    self.showing_image = true;
                                    self.current_image_url = Some(url.clone());
                                    return Ok(());
                                }
                            }
                        }
                    }
                    return Ok(());
                }
                KeyCode::Enter => {
                    // Enter键现在只用于返回，不再用于查看图片
                    self.viewing_full_post = false;
                    self.post_scroll = 0;
                    self.selected_image_button = None;
                    return Ok(());
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    // 向上滚动
                    if self.post_scroll > 0 {
                        self.post_scroll -= 1;
                    }
                    return Ok(());
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    // 向下滚动
                    self.post_scroll += 1;
                    return Ok(());
                }
                KeyCode::Tab | KeyCode::Char('i') => {
                    // 选择图片
                    if let Some(posts) = self.posts.get(&self.get_current_topic_id()) {
                        if self.selected_index < posts.len() {
                            let post = &posts[self.selected_index];
                            let image_urls = crate::core::image::extract_image_urls(&post.cooked);
                            
                            // 创建可用图片映射
                            let mut available_images = Vec::new();
                            for (i, url) in image_urls.iter().enumerate() {
                                if self.image_paths.lock().unwrap().get::<str>(url).is_some() {
                                    available_images.push((i, url.clone()));
                                }
                            }
                            
                            if !available_images.is_empty() {
                                // 选择第一个图片或切换到下一个图片
                                if self.selected_image_button.is_none() {
                                    self.selected_image_button = Some(0);
                                } else {
                                    let next_index = (self.selected_image_button.unwrap() + 1) % available_images.len();
                                    self.selected_image_button = Some(next_index);
                                }
                                return Ok(());
                            }
                        }
                    }
                }
                _ => {}
            }
            // 在完整帖子查看模式下，忽略其他按键
            return Ok(());
        }
        
        if self.input_mode {
            match key.code {
                KeyCode::Enter => {
                    self.submit_input().await?;
                    self.input_mode = false;
                    self.input.clear();
                }
                KeyCode::Esc => {
                    self.input_mode = false;
                    self.input.clear();
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                }
                _ => {}
            }
            return Ok(());
        }
        
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.navigate_back();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.navigate_up();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.navigate_next().await?;
            }
            KeyCode::Enter => {
                // 如果在设置页面且选择了 "生成 API 密钥" 选项
                if let AppTab::Settings = self.current_tab {
                    if self.selected_index == 0 {  // 第一个选项是 "生成 API 密钥"
                        self.run_api_key_generator().await?;
                        return Ok(());
                    }
                } else if let AppTab::Topic(_topic_id) = self.current_tab {
                    if let Some(posts) = self.posts.get(&self.get_current_topic_id()) {
                        if self.selected_index < posts.len() {
                            // 切换完整帖子查看状态
                            self.viewing_full_post = !self.viewing_full_post;
                            return Ok(());
                        }
                    }
                } else {
                    // 默认的导航选择逻辑
                    self.navigate_select().await?;
                }
            }
            KeyCode::Char('t') => {
                self.current_tab = AppTab::Topics;
                self.selected_index = 0;
                self.load_topics().await?;
            }
            KeyCode::Char('c') => {
                self.current_tab = AppTab::Categories;
                self.selected_index = 0;
                self.load_categories().await?;
            }
            KeyCode::Char('i') => {
                // 如果在主题中，首先确保进入完整帖子查看模式
                if let AppTab::Topic(_topic_id) = self.current_tab {
                    if let Some(posts) = self.posts.get(&self.get_current_topic_id()) {
                        if self.selected_index < posts.len() {
                            let post = &posts[self.selected_index];
                            let image_urls: Vec<String> = crate::core::image::extract_image_urls(&post.cooked);
                            
                            // 创建可用图片映射
                            let mut available_images = Vec::new();
                            for (i, url) in image_urls.iter().enumerate() {
                                if self.image_paths.lock().unwrap().get::<str>(url).is_some() {
                                    available_images.push((i, url.clone()));
                                }
                            }
                            
                            // 如果有可用图片
                            if !available_images.is_empty() {
                                // 如果还不在完整查看模式，先进入该模式
                                if !self.viewing_full_post {
                                    self.viewing_full_post = true;
                                    self.post_scroll = 0;
                                    self.selected_image_button = None;
                                    return Ok(());
                                }
                                
                                // 选择第一个图片或切换到下一个图片
                                if self.selected_image_button.is_none() {
                                    self.selected_image_button = Some(0);
                                } else {
                                    let next_index = (self.selected_image_button.unwrap() + 1) % available_images.len();
                                    self.selected_image_button = Some(next_index);
                                }
                            }
                        }
                    }
                }
            }
            KeyCode::Char('s') => {
                self.current_tab = AppTab::Settings;
                self.selected_index = 0;
            }
            KeyCode::Char('r') => {
                self.refresh_current_view().await?;
            }
            KeyCode::Char('n') => {
                self.next_page().await?;
            }
            KeyCode::Char('p') => {
                self.prev_page().await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
    
    fn navigate_back(&mut self) {
        match self.current_tab {
            AppTab::Topic(_) => {
                self.current_tab = AppTab::Topics;
                self.selected_index = 0;
                self.page = 1; // 重置页码
                // 重置图片状态
                self.selected_image_button = None;
                self.showing_image = false;
                self.current_image_url = None;
                self.viewing_full_post = false; // 重置完整帖子查看状态
            }
            AppTab::User(_) => {
                self.current_tab = AppTab::Home;
                self.selected_index = 0;
            }
            AppTab::Settings => {
                self.current_tab = AppTab::Home;
                self.selected_index = 0;
            }
            AppTab::Categories => {
                self.current_tab = AppTab::Topics;
                self.selected_index = 0;
            }
            AppTab::Topics => {
                self.current_tab = AppTab::Home;
                self.selected_index = 0;
            }
            _ => {}
        }
    }
    
    fn navigate_down(&mut self) {
        match self.current_tab {
            AppTab::Home => {
                if self.selected_index < 2 {
                    self.selected_index += 1;
                }
            }
            AppTab::Topics => {
                if !self.topics.is_empty() && self.selected_index < self.topics.len() - 1 {
                    self.selected_index += 1;
                }
            }
            AppTab::Categories => {
                if !self.categories.is_empty() && self.selected_index < self.categories.len() - 1 {
                    self.selected_index += 1;
                }
            }
            AppTab::Topic(_) => {
                if let Some(posts) = self.posts.get(&self.get_current_topic_id()) {
                    if !posts.is_empty() && self.selected_index < posts.len() - 1 {
                        self.selected_index += 1;
                        // 切换帖子时重置图片按钮状态
                        self.selected_image_button = None;
                    }
                }
            }
            AppTab::Settings => {
                // 设置页暂时没有内容
            }
            _ => {}
        }
    }
    
    fn navigate_up(&mut self) {
        match self.current_tab {
            AppTab::Home | AppTab::Topics | AppTab::Categories => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            AppTab::Topic(_) => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    // 切换帖子时重置图片按钮状态
                    self.selected_image_button = None;
                }
            }
            AppTab::Settings => {
                // 设置页暂时没有内容
            }
            _ => {}
        }
    }
    
    // 辅助方法，获取当前主题ID
    fn get_current_topic_id(&self) -> u64 {
        match self.current_tab {
            AppTab::Topic(id) => id,
            _ => 0,
        }
    }
    
    async fn navigate_select(&mut self) -> AppResult<()> {
        match &self.current_tab {
            AppTab::Home => {
                // 主页选项导航
                if self.selected_index == 0 {
                    self.current_tab = AppTab::Topics;
                    self.selected_index = 0;
                    self.load_topics().await?;
                } else if self.selected_index == 1 {
                    self.current_tab = AppTab::Categories;
                    self.selected_index = 0;
                    self.load_categories().await?;
                } else if self.selected_index == 2 {
                    self.current_tab = AppTab::Settings;
                    self.selected_index = 0;
                }
                // 重置图片状态
                self.selected_image_button = None;
                self.showing_image = false;
                self.current_image_url = None;
            }
            AppTab::Topics => {
                if !self.topics.is_empty() {
                    let topic_id = self.topics[self.selected_index].id;
                    self.current_tab = AppTab::Topic(topic_id);
                    self.selected_index = 0;
                    self.load_topic_posts(topic_id).await?;
                    // 重置图片状态
                    self.selected_image_button = None;
                    self.showing_image = false;
                    self.current_image_url = None;
                }
            }
            AppTab::Categories => {
                // 根据选定的分类加载主题
                if !self.categories.is_empty() && self.selected_index < self.categories.len() {
                    self.current_tab = AppTab::Topics;
                    self.selected_index = 0;
                    // 这里应该加载特定分类的主题，但需要扩展API客户端
                    self.load_topics().await?;
                }
            }
            AppTab::Topic(id) => {
                // 在主题中查看帖子时，选择一个用户
                if let Some(posts) = self.posts.get(id) {
                    if !posts.is_empty() && self.selected_index < posts.len() {
                        let username = posts[self.selected_index].username.clone();
                        self.current_tab = AppTab::User(username.clone());
                        self.selected_index = 0;
                        self.load_user(&username).await?;
                    }
                }
            }
            AppTab::Settings => {
                // 处理设置页面的选项
                if self.selected_index == 0 { // 生成 API 密钥
                    self.run_api_key_generator().await?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn next_page(&mut self) -> AppResult<()> {
        match self.current_tab {
            AppTab::Topics => {
                self.page += 1;
                self.selected_index = 0;
                self.load_topics().await?;
            }
            AppTab::Topic(id) => {
                self.page += 1;
                self.selected_index = 0;
                self.selected_image_button = None; // 重置图片按钮选择
                self.viewing_full_post = false; // 重置完整帖子查看状态
                self.post_scroll = 0; // 重置滚动位置
                self.load_topic_posts(id).await?;
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn prev_page(&mut self) -> AppResult<()> {
        if self.page > 1 {
            match self.current_tab {
                AppTab::Topics => {
                    self.page -= 1;
                    self.selected_index = 0;
                    self.load_topics().await?;
                }
                AppTab::Topic(id) => {
                    self.page -= 1;
                    self.selected_index = 0;
                    self.selected_image_button = None; // 重置图片按钮选择
                    self.viewing_full_post = false; // 重置完整帖子查看状态
                    self.post_scroll = 0; // 重置滚动位置
                    self.load_topic_posts(id).await?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    async fn refresh_current_view(&mut self) -> AppResult<()> {
        match self.current_tab.clone() {
            AppTab::Topics => {
                self.load_topics().await?;
            }
            AppTab::Categories => {
                self.load_categories().await?;
            }
            AppTab::Topic(id) => {
                self.load_topic_posts(id).await?;
            }
            AppTab::User(username) => {
                self.load_user(&username).await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn submit_input(&mut self) -> AppResult<()> {
        if self.input.is_empty() {
            return Ok(());
        }
        
        if let AppTab::Topic(id) = self.current_tab.clone() {
            let content = self.input.clone();
            self.client.create_post(id, &content).await?;
            self.load_topic_posts(id).await?;
        }
        
        Ok(())
    }
    
    async fn load_topics(&mut self) -> AppResult<()> {
        self.loading_state = LoadingState::Loading;
        
        match self.client.get_latest_topics(self.page).await {
            Ok(topics) => {
                self.topics = topics;
                self.loading_state = LoadingState::NotLoading;
            }
            Err(e) => {
                self.loading_state = LoadingState::Error(format!("加载主题失败: {}", e));
            }
        }
        
        Ok(())
    }
    
    async fn load_categories(&mut self) -> AppResult<()> {
        self.loading_state = LoadingState::Loading;
        
        match self.client.get_categories().await {
            Ok(categories) => {
                self.categories = categories;
                self.loading_state = LoadingState::NotLoading;
            }
            Err(e) => {
                self.loading_state = LoadingState::Error(format!("加载分类失败: {}", e));
            }
        }
        
        Ok(())
    }
    
    async fn load_topic_posts(&mut self, topic_id: u64) -> AppResult<()> {
        self.loading_state = LoadingState::Loading;
        match self.client.get_topic_posts(topic_id, self.page).await {
            Ok(posts) => {
                self.posts.insert(topic_id, posts.clone());
                self.loading_state = LoadingState::NotLoading;
                
                // 启动图片下载任务
                let image_cache = self.image_cache.clone();
                let image_paths = Arc::clone(&self.image_paths);
                
                tokio::spawn(async move {
                    for post in posts {
                        let image_urls: Vec<String> = crate::core::image::extract_image_urls(&post.cooked);
                        for url in image_urls {
                            // 检查缓存中是否已存在
                            if let Some(cached_path) = image_cache.get_cached_path(&url).await {
                                // 如果已经缓存，则更新图片路径映射
                                image_paths.lock().unwrap().insert(url, cached_path);
                                continue;
                            }
                            
                            // 下载图片
                            match crate::core::image::download_image(&url).await {
                                Ok(image_data) => {
                                    match image_cache.add_to_cache(&url, &image_data).await {
                                        Ok(path) => {
                                            // 更新图片路径映射
                                            image_paths.lock().unwrap().insert(url, path);
                                        }
                                        Err(e) => {
                                            warn!("缓存图片失败: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("下载图片失败: {}", e);
                                }
                            }
                        }
                    }
                });
                
                Ok(())
            }
            Err(e) => {
                self.loading_state = LoadingState::Error(format!("加载帖子失败: {}", e));
                Err(e.into())
            }
        }
    }
    
    async fn load_user(&mut self, username: &str) -> AppResult<()> {
        self.loading_state = LoadingState::Loading;
        
        match self.client.get_user(username).await {
            Ok(user) => {
                self.users.insert(username.to_string(), user);
                self.loading_state = LoadingState::NotLoading;
            }
            Err(e) => {
                self.loading_state = LoadingState::Error(format!("加载用户失败: {}", e));
            }
        }
        
        Ok(())
    }
    
    // 添加一个方法来处理向右导航（切换到下一个标签）
    async fn navigate_next(&mut self) -> AppResult<()> {
        match self.current_tab.clone() {
            AppTab::Home => {
                self.current_tab = AppTab::Topics;
                self.selected_index = 0;
                self.load_topics().await?;
            }
            AppTab::Topics => {
                self.current_tab = AppTab::Categories;
                self.selected_index = 0;
                self.load_categories().await?;
            }
            AppTab::Categories => {
                self.current_tab = AppTab::Settings;
                self.selected_index = 0;
            }
            _ => {}
        }
        Ok(())
    }
    
    // 添加新方法
    pub async fn run_api_key_generator(&mut self) -> AppResult<()> {
        // 保存当前终端状态
        crossterm::terminal::disable_raw_mode()?;
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;
        
        // 运行API密钥生成器
        println!("正在启动 API 密钥生成器...");
        println!("完成后将返回LDUI界面\n");
        
        if let Err(e) = crate::core::api_key_generator::run_key_generator() {
            println!("生成 API 密钥失败: {}", e);
            println!("按 Enter 键返回...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
        } else {
            println!("\nAPI 密钥已成功生成并保存到配置文件");
            println!("按 Enter 键返回...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
        }
        
        // 重新加载配置
        self.config = Config::load().map_err(|e| anyhow::anyhow!("加载配置失败: {}", e))?;
        
        // 重新创建客户端
        self.client = Arc::new(ApiClient::new(self.config.discourse.clone()));
        
        // 恢复终端状态
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
        
        // 设置需要重绘
        self.need_redraw = true;
        
        Ok(())
    }
} 