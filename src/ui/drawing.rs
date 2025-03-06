use chrono::{DateTime, Local, Utc};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap, Clear},
    Frame,
};
use crate::core::{App, AppTab, LoadingState};
use crate::ui::image_widget::ImageWidget;
use crate::core::image::extract_image_urls;
use tracing::debug;

/// 检查一行文本是否包含图片尺寸信息
fn is_image_size_info(line: &str) -> bool {
    // 检查是否包含乘号(×)和文件大小单位(KB/MB/GB)
    let has_dimension = line.contains('×');
    let has_size_unit = line.contains(" KB") || line.contains(" MB") || line.contains(" GB") || line.contains(" B ");
    
    // 如果同时包含乘号和文件大小单位，很可能是图片尺寸信息
    has_dimension && has_size_unit
}

pub fn draw_ui(f: &mut Frame, app: &App) {
    // 创建主布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(1),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.area());

    // 标题栏
    let _title = format!("LDUI - Linux Do 终端客户端 ({})", app.config.discourse.url);
    let tabs = render_tabs(app);
    f.render_widget(tabs, chunks[0]);

    // 主要内容区域
    match app.current_tab {
        AppTab::Home => draw_home(f, app, chunks[1]),
        AppTab::Topics => draw_topics(f, app, chunks[1]),
        AppTab::Categories => draw_categories(f, app, chunks[1]),
        AppTab::Topic(id) => draw_topic(f, app, id, chunks[1]),
        AppTab::User(ref username) => draw_user(f, app, username, chunks[1]),
        AppTab::Settings => draw_settings(f, app, chunks[1]),
    }

    // 底部状态栏
    draw_status_bar(f, app, chunks[2]);
    
    // 如果处于输入模式，绘制输入框
    if app.input_mode {
        draw_input(f, app);
    }
    
    // 如果显示帮助，绘制帮助窗口
    if app.show_help {
        draw_help(f);
    }
    
    // 如果正在显示图片，绘制图片
    if app.showing_image {
        draw_image(f, app);
    }
    
    // 如果正在加载，显示加载指示器
    if let LoadingState::Loading = app.loading_state {
        draw_loading(f);
    }
    
    // 如果发生错误，显示错误消息
    if let LoadingState::Error(ref error) = app.loading_state {
        draw_error(f, error);
    }
}

fn render_tabs(app: &App) -> Tabs {
    let titles = vec!["主页", "主题", "分类", "设置"];
    let selected_tab = match app.current_tab {
        AppTab::Home => 0,
        AppTab::Topics => 1,
        AppTab::Categories => 2,
        AppTab::Settings => 3,
        _ => 1, // 默认选中主题标签
    };

    let tabs: Vec<Line> = titles
        .iter()
        .map(|t| Line::from(vec![Span::styled(*t, Style::default().fg(Color::White))]))
        .collect();

    Tabs::new(tabs)
        .block(Block::default().borders(Borders::ALL).title("DisUI"))
        .select(selected_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
}

fn draw_home(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("欢迎使用 LDUI")
        .style(Style::default());
    f.render_widget(block, area);
    
    // 显示欢迎信息
    let text = vec![
        Line::from(vec![
            Span::styled("欢迎使用 ", Style::default()),
            Span::styled("LDUI", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" - ", Style::default()),
            Span::styled("Linux Do", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(" 论坛终端客户端", Style::default()),
        ]),
        Line::from(""),
        Line::from("j/↓: 向下移动选择项"),
        Line::from("k/↑: 向上移动选择项"),
        Line::from("h/←: 返回上级界面"),
        Line::from("l/→/Enter: 选择/查看详情"),
        Line::from("q: 退出程序"),
    ];
    
    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::NONE))
        .alignment(Alignment::Center);
    
    let inner_area = centered_rect(60, 40, area);
    f.render_widget(paragraph, inner_area);
}

fn draw_topics(f: &mut Frame, app: &App, area: Rect) {
    // 检查是否有主题
    if app.topics.is_empty() {
        // 如果没有主题，显示提示信息
        let message = Paragraph::new("没有可显示的主题。\n\n尝试按 'r' 刷新或 'n' 前往下一页。")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
        f.render_widget(message, area);
        return;
    }

    let items: Vec<ListItem> = app
        .topics
        .iter()
        .map(|topic| {
            let title = Line::from(vec![
                Span::styled(
                    format!("{} ", topic.title),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("[回复: {}]", topic.posts_count),
                    Style::default().fg(Color::Gray),
                ),
            ]);
            
            let created_at = format_datetime(&topic.created_at);
            let info = Line::from(vec![
                Span::styled(
                    format!("创建于: {} ", created_at),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("浏览: {}", topic.views),
                    Style::default().fg(Color::Gray),
                ),
            ]);
            
            let tags = if let Some(ref tags) = topic.tags {
                if !tags.is_empty() {
                    Line::from(vec![Span::styled(
                        format!("标签: {}", tags.join(", ")),
                        Style::default().fg(Color::Cyan),
                    )])
                } else {
                    Line::default()
                }
            } else {
                Line::default()
            };
            
            ListItem::new(vec![title, info, tags])
        })
        .collect();

    let topics_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!("主题 (第{}页)", app.page)))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(topics_list, area, &mut state);
    
    // 添加提示信息
    let hint_text = "按 Enter 查看帖子完整内容，j/k 或 ↓/↑ 选择帖子，n/p 切换页面";
    let hint = Paragraph::new(hint_text)
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center);
        
    let hint_area = Rect {
        x: area.x,
        y: area.height.saturating_sub(2) + area.y,
        width: area.width,
        height: 1,
    };
    
    f.render_widget(hint, hint_area);
}

fn draw_categories(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .categories
        .iter()
        .map(|category| {
            let title = Line::from(vec![
                Span::styled(
                    format!("{} ", category.name),
                    Style::default()
                        .fg(parse_color(&category.color))
                        .add_modifier(Modifier::BOLD),
                ),
            ]);
            
            let info = Line::from(vec![
                Span::styled(
                    format!("主题: {} ", category.topic_count),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("帖子: {}", category.post_count),
                    Style::default().fg(Color::Gray),
                ),
            ]);
            
            let description = if let Some(ref desc) = category.description {
                Line::from(vec![Span::styled(
                    desc.clone(),
                    Style::default().fg(Color::White),
                )])
            } else {
                Line::default()
            };
            
            ListItem::new(vec![title, info, description])
        })
        .collect();

    let categories_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("分类"))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(categories_list, area, &mut state);
}

fn draw_topic(f: &mut Frame, app: &App, id: u64, area: Rect) {
    let inner_area = Block::default()
        .borders(Borders::ALL)
        .title(format!("主题 #{}", id))
        .inner(area);
    
    if let Some(posts) = app.posts.get(&id) {
        // 如果处于完整帖子查看模式
        if app.viewing_full_post && app.selected_index < posts.len() {
            let post = &posts[app.selected_index];
            
            // 提取图片URL
            let image_urls = extract_image_urls(&post.cooked);
            
            // 创建可用图片映射
            let mut available_images = Vec::new();
            for (i, url) in image_urls.iter().enumerate() {
                if app.image_paths.lock().unwrap().get::<str>(url).is_some() {
                    available_images.push((i, url.clone()));
                }
            }
            
            // 创建帖子头部信息
            let title = format!("帖子 #{} - {}", post.id, post.username);
            
            // 简单清理HTML标签
            let mut cleaned = post.cooked.clone();
            
            // 替换一些常见HTML标签为纯文本等价物
            cleaned = cleaned.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
            cleaned = cleaned.replace("<p>", "").replace("</p>", "\n");
            cleaned = cleaned.replace("<strong>", "").replace("</strong>", "");
            cleaned = cleaned.replace("<em>", "").replace("</em>", "");
            cleaned = cleaned.replace("&nbsp;", " ");
            cleaned = cleaned.replace("&lt;", "<").replace("&gt;", ">");
            cleaned = cleaned.replace("&quot;", "\"").replace("&apos;", "'");
            cleaned = cleaned.replace("&amp;", "&");
            
            // 移除可能的剩余HTML标签 (简单实现，不使用regex)
            let mut result = String::with_capacity(cleaned.len());
            let mut in_tag = false;
            
            for c in cleaned.chars() {
                if c == '<' {
                    in_tag = true;
                } else if c == '>' {
                    in_tag = false;
                } else if !in_tag {
                    result.push(c);
                }
            }
            
            let content_text = result;
            
            // 处理连续换行符，将多个换行符替换为一个
            let mut processed_text = String::new();
            let mut last_char_was_newline = false;
            
            for c in content_text.chars() {
                if c == '\n' {
                    if !last_char_was_newline {
                        processed_text.push(c);
                    }
                    last_char_was_newline = true;
                } else {
                    processed_text.push(c);
                    last_char_was_newline = false;
                }
            }
            
            // 提取图片URL
            let image_urls = extract_image_urls(&post.cooked);
            let _has_images = !image_urls.is_empty() && image_urls.iter().any(|url| {
                app.image_paths.lock().unwrap().get(url).is_some()
            });
            
            // 将内容按行分割并过滤掉图片尺寸信息行
            let lines_iter = processed_text.split('\n')
                .filter(|line| !is_image_size_info(line));
            let mut content_lines = Vec::new();
            
            // 创建可用图片映射
            let mut available_images = Vec::new();
            for (i, url) in image_urls.iter().enumerate() {
                if app.image_paths.lock().unwrap().get::<str>(url).is_some() {
                    available_images.push((i, url.clone()));
                }
            }
            
            // 创建一个简单的映射来找到图片可能在的行号
            // 这只是一个近似，因为HTML处理后不容易精确定位
            let mut img_positions = Vec::new();
            
            // 计算内容总行数
            let total_lines = content_text.lines().count();
            
            // 为每个图片分配一个位置 - 采用更精确的定位方法
            if !available_images.is_empty() && total_lines > 0 {
                // 尝试查找原始HTML中的图片标签位置，并映射到处理后的文本
                let raw_html = &post.cooked;
                let _line_counter = 0;
                let _html_pos = 0;
                
                // 创建一个简单的映射来将原始HTML位置转换为处理后的文本行号
                let mut img_tag_positions = Vec::new();
                
                // 查找所有img标签位置
                for (idx, url) in image_urls.iter().enumerate() {
                    if let Some(pos) = raw_html.find(&format!("src=\"{}\"", url)) {
                        img_tag_positions.push((idx, pos));
                    }
                }
                
                // 按HTML中的位置排序
                img_tag_positions.sort_by_key(|&(_, pos)| pos);
                
                if img_tag_positions.is_empty() {
                    // 如果无法找到精确位置，退回到均匀分布
                    let spacing = total_lines / (available_images.len() + 1);
                    let spacing = spacing.max(3); // 至少间隔3行
                    
                    for i in 0..available_images.len() {
                        let pos = (i + 1) * spacing;
                        if pos < total_lines {
                            img_positions.push(pos);
                        }
                    }
                } else {
                    // 将HTML位置比例映射到文本行
                    let html_length = raw_html.len();
                    
                    for (idx, html_pos) in img_tag_positions {
                        // 确保这个URL是可用的
                        if available_images.iter().any(|(i, _)| *i == idx) {
                            // 计算相对位置并映射到行号
                            let relative_pos = html_pos as f64 / html_length as f64;
                            let line_pos = (relative_pos * total_lines as f64) as usize;
                            let line_pos = line_pos.min(total_lines - 1);
                            img_positions.push(line_pos);
                        }
                    }
                }
                
                debug!("图片位置列表: {:?}", img_positions);
                debug!("可用图片列表: {}", available_images.len());
            }
            
            // 内容行计数器
            let mut content_line_counter = 0;
            let mut img_counter = 0;
            
            // 添加帖子内容
            for line in lines_iter {
                // 空行处理
                if line.trim().is_empty() {
                    content_lines.push(Line::from(Span::raw("")));
                    content_line_counter += 1;
                    continue;
                }
                
                // 检查这一行是否应该放置图片按钮
                if img_counter < img_positions.len() && content_line_counter >= img_positions[img_counter] {
                    // 在合适的位置插入图片按钮
                    let button_style = if Some(img_counter) == app.selected_image_button {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::ITALIC)
                    };
                    
                    let button_text = format!("[{} 图片 #{} (按o键查看)]", 
                        if Some(img_counter) == app.selected_image_button { "✓" } else { " " }, 
                        img_counter + 1
                    );
                    content_lines.push(Line::from(Span::styled(button_text, button_style)));
                    img_counter += 1;
                }
                
                // 正常内容行
                content_lines.push(Line::from(Span::raw(line)));
                content_line_counter += 1;
            }
            
            // 计算内容实际行数与可见区域行数的差值，用于限制滚动范围
            let content_height = content_lines.len() as u16;
            let visible_area_height = inner_area.height.saturating_sub(2); // 减去边框
            
            // 调整滚动位置，避免无效滚动
            let max_scroll = if content_height > visible_area_height {
                content_height - visible_area_height
            } else {
                0
            };
            
            // 确保不会滚动超出内容
            let adjusted_scroll = app.post_scroll.min(max_scroll as u16);
            
            // 创建并渲染帖子内容
            let full_post_view = Paragraph::new(content_lines)
                .block(Block::default().borders(Borders::ALL).title(title))
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White))
                .scroll((adjusted_scroll, 0));  // 使用调整后的滚动值
                
            f.render_widget(full_post_view, inner_area);
            
            // 在底部添加提示
            let hint_text = "按 ↑/↓/j/k 键滚动内容，Tab/i 选择图片，o 查看图片，Enter/Esc 返回";
            
            let hint = Paragraph::new(hint_text)
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
                
            let hint_area = Rect {
                x: area.x,
                y: area.height.saturating_sub(2) + area.y,
                width: area.width,
                height: 1,
            };
            
            f.render_widget(hint, hint_area);
            
            return; // 完整帖子查看模式下，不渲染其他内容
        }
        
        // 非完整帖子查看模式下的渲染逻辑
        let items: Vec<ListItem> = posts
            .iter()
            .map(|post| {
                // 创建帖子头部信息
                let header = Line::from(vec![
                    Span::styled(
                        format!("{} ", post.username),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format_datetime(&post.created_at),
                        Style::default().fg(Color::Gray),
                    ),
                ]);
                
                // 将HTML内容分割成多行，以便在终端中能够正确显示
                let content_width = inner_area.width.saturating_sub(2) as usize; // 减去内边距
                let mut content_lines = Vec::new();
                
                // 简单清理HTML标签
                let mut cleaned = post.cooked.clone();
                
                // 替换一些常见HTML标签为纯文本等价物
                cleaned = cleaned.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");
                cleaned = cleaned.replace("<p>", "").replace("</p>", "\n");
                cleaned = cleaned.replace("<strong>", "").replace("</strong>", "");
                cleaned = cleaned.replace("<em>", "").replace("</em>", "");
                cleaned = cleaned.replace("&nbsp;", " ");
                cleaned = cleaned.replace("&lt;", "<").replace("&gt;", ">");
                cleaned = cleaned.replace("&quot;", "\"").replace("&apos;", "'");
                cleaned = cleaned.replace("&amp;", "&");
                
                // 移除可能的剩余HTML标签 (简单实现，不使用regex)
                let mut result = String::with_capacity(cleaned.len());
                let mut in_tag = false;
                
                for c in cleaned.chars() {
                    if c == '<' {
                        in_tag = true;
                    } else if c == '>' {
                        in_tag = false;
                    } else if !in_tag {
                        result.push(c);
                    }
                }
                
                let content_text = result;
                
                // 处理连续换行符，将多个换行符替换为一个
                let mut processed_text = String::new();
                let mut last_char_was_newline = false;
                
                for c in content_text.chars() {
                    if c == '\n' {
                        if !last_char_was_newline {
                            processed_text.push(c);
                        }
                        last_char_was_newline = true;
                    } else {
                        processed_text.push(c);
                        last_char_was_newline = false;
                    }
                }
                
                // 提取图片URL
                let image_urls = extract_image_urls(&post.cooked);
                let _has_images = !image_urls.is_empty() && image_urls.iter().any(|url| {
                    app.image_paths.lock().unwrap().get(url).is_some()
                });
                
                // 将内容按行分割并过滤掉图片尺寸信息行
                let lines_iter = processed_text.split('\n')
                    .filter(|line| !is_image_size_info(line));
                let mut lines_count = 0;
                let max_preview_lines = 5; // 设置预览时最多显示的行数
                
                for line in lines_iter {
                    if line.trim().is_empty() {
                        content_lines.push(Line::from(Span::raw("")));
                        lines_count += 1;
                        if lines_count >= max_preview_lines {
                            break;
                        }
                        continue;
                    }
                    
                    // 长行处理 - 按照终端宽度自动分割长行
                    if line.len() > content_width {
                        let chars: Vec<char> = line.chars().collect();
                        let mut current_pos = 0;
                        
                        while current_pos < chars.len() {
                            let end_pos = std::cmp::min(current_pos + content_width, chars.len());
                            let segment: String = chars[current_pos..end_pos].iter().collect();
                            content_lines.push(Line::from(Span::raw(segment)));
                            
                            lines_count += 1;
                            if lines_count >= max_preview_lines {
                                break;
                            }
                            
                            current_pos = end_pos;
                        }
                        
                        if lines_count >= max_preview_lines {
                            break;
                        }
                    } else {
                        content_lines.push(Line::from(Span::raw(line.to_string())));
                        lines_count += 1;
                        if lines_count >= max_preview_lines {
                            break;
                        }
                    }
                }
                
                // 如果内容被截断了或者有图片，添加省略号提示
                let has_more_content = processed_text.split('\n').count() > lines_count || 
                                      (processed_text.len() > content_width * lines_count);
                
                if has_more_content || _has_images {
                    let mut prompt = "... 按 Enter 查看完整内容".to_string();
                    if _has_images {
                        prompt += " 和图片附件";
                    }
                    prompt += " ...";
                    
                    content_lines.push(Line::from(Span::styled(
                        prompt,
                        Style::default().fg(Color::Yellow),
                    )));
                }
                
                // 创建分割线
                let separator = Line::from(Span::styled(
                    "─".repeat(inner_area.width as usize), 
                    Style::default().fg(Color::DarkGray),
                ));
                
                // 组合成完整的帖子显示
                let mut all_lines = vec![header, Line::default()];
                
                // 只有在内容行不为空时才添加
                if !content_lines.is_empty() {
                    all_lines.extend(content_lines);
                } else {
                    // 如果内容为空，添加一个提示
                    all_lines.push(Line::from(Span::styled(
                        "[无内容]",
                        Style::default().fg(Color::Gray),
                    )));
                }
                
                all_lines.push(separator);
                
                ListItem::new(all_lines)
            })
            .collect();

        let topic_title = if let Some(topic) = app.topics.iter().find(|t| t.id == id) {
            topic.title.clone()
        } else {
            format!("主题 #{}", id)
        };

        let posts_list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(format!("{} (第{}页)", topic_title, app.page)))
            .highlight_style(
                Style::default()
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut state = ListState::default();
        
        // 处理选中索引逻辑
        if app.selected_index >= posts.len() && !posts.is_empty() {
            state.select(Some(0)); // 如果选中索引超出范围，选择第一个
        } else if !posts.is_empty() {
            state.select(Some(app.selected_index));
        } else {
            state.select(None); // 空列表不选择任何项
        }
        
        // 渲染帖子列表
        f.render_stateful_widget(posts_list, area, &mut state);
        
        // 添加提示信息
        let hint_text = "按 Enter 查看帖子完整内容，j/k 或 ↓/↑ 选择帖子，n/p 切换页面";
        let hint = Paragraph::new(hint_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center);
            
        let hint_area = Rect {
            x: area.x,
            y: area.height.saturating_sub(2) + area.y,
            width: area.width,
            height: 1,
        };
        
        f.render_widget(hint, hint_area);
    } else {
        // 正在加载帖子
        let paragraph = Paragraph::new("正在加载帖子...")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, inner_area);
    }
}

fn draw_user(f: &mut Frame, app: &App, username: &str, area: Rect) {
    if let Some(user) = app.users.get(username) {
        let text = vec![
            Line::from(vec![
                Span::styled(
                    format!("用户名: ", ),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    user.username.clone(),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("名称: "),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    user.name.clone().unwrap_or_else(|| "-".to_string()),
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(vec![
                Span::styled(
                    format!("信任等级: "),
                    Style::default().fg(Color::Gray),
                ),
                Span::styled(
                    format!("{}", user.trust_level),
                    Style::default().fg(Color::White),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title(format!("用户: {}", username)))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new("正在加载用户信息...")
            .block(Block::default().borders(Borders::ALL).title(format!("用户: {}", username)))
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }
}

fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    // 分割区域为标题信息区和选项区
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Min(6),  // 信息区
                Constraint::Length(3),  // 选项区
            ]
            .as_ref(),
        )
        .split(area);
    
    // 信息区域
    let text = vec![
        Line::from(vec![
            Span::styled(
                "Linux Do URL: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(app.config.discourse.url.clone()),
        ]),
        Line::from(vec![
            Span::styled(
                "API 密钥: ",
                Style::default().fg(Color::Gray),
            ),
            Span::styled(
                if app.config.discourse.api_key.is_empty() { 
                    "未设置".to_string() 
                } else { 
                    "已设置 (已隐藏)".to_string() 
                },
                if app.config.discourse.api_key.is_empty() {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                },
            ),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("设置信息"))
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, chunks[0]);
    
    // 选项区域
    let options = vec![
        "生成 API 密钥",
    ];
    
    let options_list = List::new(options.iter().map(|&o| ListItem::new(o)).collect::<Vec<_>>())
        .block(Block::default().borders(Borders::ALL).title("操作"))
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(options_list, chunks[1], &mut state);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let current_view = match &app.current_tab {
        AppTab::Home => "主页".to_string(),
        AppTab::Topics => "主题".to_string(),
        AppTab::Categories => "分类".to_string(),
        AppTab::Topic(id) => format!("主题 #{}", id),
        AppTab::User(username) => format!("用户: {}", username),
        AppTab::Settings => "设置".to_string(),
    };

    let help_text = "按 '?' 查看帮助";
    let page_info = if matches!(app.current_tab, AppTab::Topics | AppTab::Topic(_)) {
        format!("第 {} 页", app.page)
    } else {
        "".to_string()
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{} ", current_view),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", page_info),
            Style::default().fg(Color::Gray),
        ),
        Span::styled(
            help_text,
            Style::default().fg(Color::Blue),
        ),
    ]))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(status, area);
}

fn draw_input(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, f.area());
    let input = Paragraph::new(app.input.as_ref() as &str)
        .block(Block::default().borders(Borders::ALL).title("输入回复"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(input, area);
}

fn draw_help(f: &mut Frame) {
    let area = centered_rect(60, 20, f.area());
    let help_text = vec![
        Line::from("导航:"),
        Line::from("  j/↓: 向下移动"),
        Line::from("  k/↑: 向上移动"),
        Line::from("  h/←: 返回"),
        Line::from("  l/→/Enter: 选择/查看详情"),
        Line::from(""),
        Line::from("在查看帖子时:"),
        Line::from("  Enter: 切换完整帖子查看模式"),
        Line::from("  ↑/↓: 在完整帖子中滚动"),
        Line::from("  Esc: 退出完整帖子查看模式"),
        Line::from(""),
        Line::from("功能:"),
        Line::from("  t: 查看主题"),
        Line::from("  c: 查看分类"),
        Line::from("  s: 设置"),
        Line::from("  r: 刷新"),
        Line::from("  n: 下一页"),
        Line::from("  p: 上一页"),
        Line::from("  q: 退出"),
        Line::from(""),
        Line::from("按任意键关闭此帮助"),
    ];
    let help = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("帮助"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });
    f.render_widget(help, area);
}

fn draw_loading(f: &mut Frame) {
    let area = centered_rect(30, 3, f.area());
    let loading = Paragraph::new("加载中...")
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    f.render_widget(loading, area);
}

fn draw_error(f: &mut Frame, error: &str) {
    let area = centered_rect(60, 5, f.area());
    let error_text = Paragraph::new(error)
        .block(Block::default().borders(Borders::ALL).title("错误"))
        .style(Style::default().fg(Color::Red))
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(error_text, area);
}

// 创建一个居中的矩形
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

// 格式化日期时间
fn format_datetime(dt: &DateTime<Utc>) -> String {
    let local_time = dt.with_timezone(&Local);
    local_time.format("%Y-%m-%d %H:%M").to_string()
}

// 解析颜色字符串为Tui颜色
fn parse_color(color_str: &str) -> Color {
    match color_str.trim_start_matches('#') {
        "ff0000" => Color::Red,
        "00ff00" => Color::Green,
        "0000ff" => Color::Blue,
        "ffff00" => Color::Yellow,
        "ff00ff" => Color::Magenta,
        "00ffff" => Color::Cyan,
        "ffffff" => Color::White,
        _ => Color::Gray,
    }
}

// 在文件末尾添加新函数
fn draw_image(f: &mut Frame, app: &App) {
    if let Some(url) = &app.current_image_url {
        debug!("尝试渲染图片: {}", url);
        // 使用clone避免长时间持有锁
        let image_path = app.image_paths.lock().unwrap().get(url).cloned();
        
        if let Some(path) = image_path {
            debug!("找到图片路径: {:?}", path);
            // 创建占满整个屏幕的清除层，确保图片显示在最上层
            f.render_widget(Clear, f.area());
            
            // 添加半透明背景
            let bg_block = Block::default()
                .style(Style::default().bg(Color::Rgb(0, 0, 0)));
            f.render_widget(bg_block, f.area());
            
            // 计算一个更合适的图片显示区域（根据屏幕大小按比例调整）
            let screen_width = f.area().width;
            let screen_height = f.area().height;
            
            // 为大屏幕使用更大的显示区域，但限制最大尺寸
            let percent_x = if screen_width > 100 { 90 } else { 80 };
            let percent_y = if screen_height > 50 { 80 } else { 70 };
            
            let image_area = centered_rect(percent_x, percent_y, f.area());
            debug!("图片显示区域: {:?}", image_area);
            
            // 先渲染边框和背景
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(Span::styled("图片预览", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                .title_alignment(Alignment::Center)
                .style(Style::default().bg(Color::Black));
                
            f.render_widget(block.clone(), image_area);
            
            // 计算内部区域并渲染图片
            let inner_area = block.inner(image_area);
            
            // 计算图片显示区域（占内部区域的上部分）
            let img_display_height = inner_area.height.saturating_sub(8); // 留出底部空间显示链接信息
            let img_area = Rect {
                x: inner_area.x,
                y: inner_area.y,
                width: inner_area.width,
                height: img_display_height,
            };
            
            // 清除内部区域，防止透明区域堆叠问题
            f.render_widget(Clear, inner_area);
            
            // 检查文件是否存在
            if !path.exists() {
                let error_message = format!("图片文件不存在: {:?}", path);
                let error_text = Paragraph::new(error_message)
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center);
                f.render_widget(error_text, inner_area);
                debug!("图片文件不存在: {:?}", path);
                return;
            }
            
            debug!("开始渲染图片: {:?}", path);
            // 创建并渲染图片组件
            let image_widget = ImageWidget::new(path)
                .max_width(img_area.width)
                .max_height(img_area.height)
                .maintain_aspect_ratio(true);
            
            f.render_widget(image_widget, img_area);
            
            // 显示链接信息
            let link_info = format!("链接: {}", url);
            let link_area = Rect {
                x: inner_area.x + 1,
                y: inner_area.y + img_display_height + 1,
                width: inner_area.width.saturating_sub(2),
                height: 2,
            };
            
            // 添加链接分割线
            let separator = Line::from(Span::styled(
                "─".repeat(link_area.width as usize),
                Style::default().fg(Color::DarkGray),
            ));
            
            // 创建链接信息部分
            let link_paragraph = Paragraph::new(vec![
                separator,
                Line::from(Span::styled(link_info, Style::default().fg(Color::Cyan))),
            ])
            .alignment(Alignment::Center);
            
            f.render_widget(link_paragraph, link_area);
            
            // 在底部添加操作提示
            let hint_text = "按 Enter、Esc 或 o 键返回";
            
            let hint = Paragraph::new(hint_text)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
                
            let hint_area = Rect {
                x: f.area().x,
                y: f.area().height.saturating_sub(2) + f.area().y,
                width: f.area().width,
                height: 1,
            };
            
            f.render_widget(hint, hint_area);
        } else {
            debug!("未找到图片路径: {}", url);
        }
    } else {
        debug!("没有当前图片URL");
    }
} 