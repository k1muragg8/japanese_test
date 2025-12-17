# 日语假名导师 (Kana Tutor)

一个基于间隔重复系统 (Spaced Repetition System, SRS) 的日语假名学习工具。

**现已支持两种模式:**
- **终端模式 (TUI)**: 经典命令行学习体验。
- **网页模式 (WASM)**: 一个由 **Leptos** 和 **Axum** 驱动的，设计现代、简约、扁平化的网页界面。

## 环境准备 (Prerequisites)

在运行网页版之前，用户需要安装 WASM 构建工具：

```bash
# 安装 Rust (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 添加 WebAssembly 编译目标
rustup target add wasm32-unknown-unknown

# 安装 Trunk (WASM 打包工具)
cargo install trunk
```

## 🚀 如何运行 (Usage)

### 🖥️ 终端模式 (TUI)
直接使用 Cargo 运行项目即可：
```bash
cargo run --release
```
程序将启动终端界面。请使用键盘进行操作。

### 🌐 网页模式 (WASM)
要启动网页界面，首先需要编译前端资源，然后运行后端服务器。

1.  **构建前端:**
    ```bash
    cd frontend
    trunk build --release
    cd ..
    ```

2.  **运行服务器:**
    ```bash
    cargo run --release -- --web
    ```

3.  **打开应用:**
    在浏览器中访问 [http://0.0.0.0:3000](http://0.0.0.0:3000)。

## ⌨️ 操作指南 (Controls)

### 网页界面
网页界面采用了 **“无按钮，仅回车”** 的工作流：
- **输入答案**: 直接开始打字，输入框会自动聚焦。
- **提交**: 按 **回车 (Enter)**。
- **下一张卡片**: 再次按 **回车 (Enter)**。

### 终端界面
- **[Enter]**: 开始测验 / 提交答案 / 下一张卡片。
- **[Esc]** 或 **`q`**: 退出。

## 🧠 间隔重复系统 (SRS)
两种模式共享同一个 SQLite 数据库 (`kana.db`)。本应用使用简化的 SM2 算法来安排复习计划，确保您能专注于练习您觉得困难的假名。