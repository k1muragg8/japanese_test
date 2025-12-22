use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use dotenvy::dotenv;
// 【新增引用】
use tower_http::services::ServeDir;

mod api;
mod app;
mod data;
mod db;
mod feedback;
// mod ui; // Removed

use crate::app::App;
use crate::api::{app_router, ApiState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    env_logger::init();

    // Initialize App (which inits DB)
    let app_logic = App::new().await?;
    let app_state = Arc::new(Mutex::new(app_logic));

    let api_state = ApiState {
        app: app_state,
    };

    // 【修改这里】
    // 原来的代码：let app = app_router(api_state);
    // 现在的代码：把 API 路由和静态文件服务连起来
    let app = app_router(api_state)
        .fallback_service(ServeDir::new("frontend/dist")); // <--- 关键！找不到的路径都去 frontend 找

    // let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let addr = SocketAddr::from(([192, 168, 25, 76], 3000));
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}