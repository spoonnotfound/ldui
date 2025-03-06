mod core;
mod api;
mod ui;

use std::io;
use std::time::Duration;
use std::env;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tracing::error;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use core::{App, AppResult, Config, initialize_logging, run_key_generator};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    // 设置日志
    initialize_logging()?;
    
    // 检查命令行参数
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args[1] == "--generate-api-key" || args[1] == "-g" {
            // 运行 API 密钥生成器
            println!("启动 Linux Do API 密钥生成器");
            if let Err(e) = run_key_generator() {
                eprintln!("生成 API 密钥失败: {}", e);
                return Err(color_eyre::eyre::eyre!("生成 API 密钥失败: {}", e));
            }
            return Ok(());
        } else if args[1] == "--help" || args[1] == "-h" {
            // 显示帮助信息
            println!("LDUI - Linux Do 论坛终端界面");
            println!();
            println!("用法:");
            println!("  ldui                     启动 LDUI 终端界面");
            println!("  ldui --generate-api-key  启动 API 密钥生成器");
            println!("  ldui -g                  启动 API 密钥生成器 (简写)");
            println!("  ldui --help              显示此帮助信息");
            println!("  ldui -h                  显示此帮助信息 (简写)");
            return Ok(());
        }
    }
    
    // 加载配置
    let config = Config::load()?;
    
    // 检查配置中是否设置了 API Key
    if !config.has_valid_api_key() {
        println!("检测到 API Key 未设置，正在启动 API Key 生成器...");
        if let Err(e) = run_key_generator() {
            eprintln!("生成 API Key 失败: {}", e);
            return Err(color_eyre::eyre::eyre!("生成 API Key 失败: {}", e));
        }
        // 重新加载配置
        let _config = Config::load()?;
    }
    
    // 设置终端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 创建应用状态
    let mut app = App::new(config);
    
    // 运行应用
    let res = run_app(&mut terminal, &mut app).await;

    // 恢复终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> AppResult<()> {
    let tick_rate = Duration::from_millis(1000);
    let mut last_tick = std::time::Instant::now();
    
    // 初始化应用
    app.init().await?;

    loop {
        terminal.draw(|f| ui::draw_ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') && app.should_quit() {
                    // 在退出前确保屏幕是干净的
                    terminal.clear()?;
                    return Ok(());
                }
                app.handle_key_event(key).await?;
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.tick().await?;
            last_tick = std::time::Instant::now();
        }
        
        // 检查是否需要额外刷新屏幕（例如，清除图片残留）
        if app.need_redraw {
            // 强制清屏并重绘
            terminal.clear()?;
            terminal.draw(|f| ui::draw_ui(f, app))?;
            app.need_redraw = false; // 重置标志
        }
    }
}
