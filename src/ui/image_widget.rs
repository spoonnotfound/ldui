use std::path::Path;
use ratatui::{
    widgets::{Block, Widget},
    layout::Rect,
    buffer::Buffer,
    style::{Color, Style},
};
use ratatui_image::{
    StatefulImage, Resize, FilterType,
    picker::Picker,
};
use image::ImageReader;
use tracing::{debug, warn};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use lazy_static::lazy_static;

// 使用静态缓存存储已处理的图片数据
lazy_static! {
    static ref IMAGE_CACHE: Arc<RwLock<HashMap<String, Vec<u8>>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// 图片组件，使用ratatui-image库在终端中渲染图片
pub struct ImageWidget {
    pub path: String,
    pub block: Option<Block<'static>>,
    pub max_width: Option<u16>,
    pub max_height: Option<u16>,
    pub maintain_aspect_ratio: bool,
}

impl ImageWidget {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            path: path.as_ref().to_string_lossy().to_string(),
            block: None,
            max_width: None,
            max_height: None,
            maintain_aspect_ratio: true,
        }
    }

    // 清除指定区域的图片
    pub fn clear_area(area: Rect, buf: &mut Buffer) {
        // 使用 ratatui 的方式来清除区域
        // 用空白字符填充整个区域
        for y in area.y..area.y+area.height {
            for x in area.x..area.x+area.width {
                if y < buf.area.bottom() && x < buf.area.right() {
                    // 将每个单元格设置为空白
                    buf[(x, y)].set_char(' ');
                }
            }
        }
    }
    
    pub fn max_width(mut self, width: u16) -> Self {
        self.max_width = Some(width);
        self
    }
    
    pub fn max_height(mut self, height: u16) -> Self {
        self.max_height = Some(height);
        self
    }
    
    pub fn maintain_aspect_ratio(mut self, maintain: bool) -> Self {
        self.maintain_aspect_ratio = maintain;
        self
    }

    // 添加图片缓存检查方法
    fn get_cached_data(&self) -> Option<Vec<u8>> {
        if let Ok(cache) = IMAGE_CACHE.read() {
            return cache.get(&self.path).cloned();
        }
        None
    }

    // 添加图片缓存保存方法
    fn cache_data(&self, data: Vec<u8>) {
        if let Ok(mut cache) = IMAGE_CACHE.write() {
            // 限制缓存大小，避免内存泄漏 (最多缓存10张图片)
            if cache.len() > 10 {
                // 简单实现：清空缓存
                cache.clear();
            }
            cache.insert(self.path.clone(), data);
        }
    }
}

impl Widget for ImageWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // 处理边框
        let inner_area = match self.block {
            Some(ref block) => {  // 使用ref关键字来借用而不是移动
                let inner = block.inner(area);
                block.render(area, buf);
                inner
            }
            None => area,
        };
        
        // 检查区域大小，如果太小就不渲染
        if inner_area.width < 10 || inner_area.height < 5 {
            debug!("区域太小，跳过图片渲染");
            return;
        }
        
        // 尝试从缓存获取图片数据
        if let Some(data) = self.get_cached_data() {
            debug!("使用缓存的图片数据: {} 字节", data.len());
            self.render_from_data(data, inner_area, buf);
            return;
        }
        
        // 首先检查文件是否存在
        if !std::path::Path::new(&self.path).exists() {
            render_error(&format!("图片文件不存在: {}", self.path), inner_area, buf);
            return;
        }
        
        // 尝试读取文件
        match std::fs::read(&self.path) {
            Ok(data) => {
                debug!("成功读取文件数据: {} 字节", data.len());
                
                // 缓存图片数据
                self.cache_data(data.clone());
                
                // 渲染图片
                self.render_from_data(data, inner_area, buf);
            },
            Err(e) => {
                // 无法读取文件
                debug!("无法读取图片文件: {} (路径: {})", e, self.path);
                render_error(&format!("无法读取图片文件: {}", e), inner_area, buf);
            }
        }
    }
}

// 将图片渲染逻辑分离为单独的方法
impl ImageWidget {
    fn render_from_data(&self, data: Vec<u8>, inner_area: Rect, buf: &mut Buffer) {
        // 首先清除渲染区域，避免透明区域堆叠问题
        Self::clear_area(inner_area, buf);
        
        // 尝试确定图片格式
        let format = match std::path::Path::new(&self.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("") {
                "jpg" | "jpeg" => Some(image::ImageFormat::Jpeg),
                "png" => Some(image::ImageFormat::Png),
                "gif" => Some(image::ImageFormat::Gif),
                "webp" => Some(image::ImageFormat::WebP),
                "bmp" => Some(image::ImageFormat::Bmp),
                "ico" => Some(image::ImageFormat::Ico),
                "tiff" | "tif" => Some(image::ImageFormat::Tiff),
                _ => None
            }.or_else(|| {
                // 如果无法通过扩展名确定，尝试猜测格式
                match image::guess_format(&data) {
                    Ok(format) => Some(format),
                    Err(e) => {
                        warn!("无法猜测图片格式: {}", e);
                        None
                    }
                }
            });
        
        // 检查是否成功确定了格式
        if let Some(img_format) = format {
            // 使用确定的格式解码图片
            match ImageReader::with_format(std::io::Cursor::new(data), img_format).decode() {
                Ok(img) => {
                    // 计算适合的宽高，限制最大尺寸以减轻处理负担
                    let _width = self.max_width.unwrap_or(inner_area.width).min(200);
                    let _height = self.max_height.unwrap_or(inner_area.height).min(100);
                    
                    // 创建一个固定字体大小的Picker
                    let picker = Picker::from_fontsize((8, 16));
                    
                    // 创建协议
                    let mut protocol = picker.new_resize_protocol(img);
                    
                    // 使用更高效的缩放算法
                    let resize_mode = if self.maintain_aspect_ratio {
                        Resize::Fit(Some(FilterType::Nearest))  // 改为Nearest算法，更高效
                    } else {
                        Resize::Scale(Some(FilterType::Nearest))
                    };
                    
                    // 使用更高级的配置创建图像组件
                    let image_widget = StatefulImage::default()
                        .resize(resize_mode);
                    
                    // 确保区域有效
                    if inner_area.width > 0 && inner_area.height > 0 {
                        // 使用StatefulWidget::render方法渲染图像
                        ratatui::widgets::StatefulWidget::render(image_widget, inner_area, buf, &mut protocol);
                    }
                },
                Err(e) => {
                    // 图片解码失败显示错误
                    debug!("图片解码失败: {} (路径: {})", e, self.path);
                    render_error(&format!("图片解码失败: {}", e), inner_area, buf);
                }
            }
        } else {
            // 无法确定图片格式
            render_error(&format!("无法确定图片格式"), inner_area, buf);
        }
    }
}

// 渲染错误信息的辅助函数
fn render_error(message: &str, area: Rect, buf: &mut Buffer) {
    let x = area.x + (area.width.saturating_sub(message.len() as u16)) / 2;
    let y = area.y + area.height / 2;
    
    if y < buf.area.bottom() && x < buf.area.right() {
        buf.set_string(
            x,
            y,
            message,
            Style::default().fg(Color::Red),
        );
    }
} 