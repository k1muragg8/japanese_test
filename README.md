# Kana Tutor

A Japanese Kana learning tool powered by a Spaced Repetition System (SRS).

**Now featuring two modes:**
- **Terminal Mode (TUI)**: The classic command-line experience.
- **Web Mode (WASM)**: A modern, minimalistic, flat-design web interface powered by **Leptos** and **Axum**.

## Prerequisites

Users need to install the WASM build tools before running the web version:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WebAssembly target
rustup target add wasm32-unknown-unknown

# Install Trunk (WASM bundler)
cargo install trunk
```

## 🚀 Usage

### 🖥️ Terminal Mode (TUI)
Simply run the project using Cargo:
```bash
cargo run --release
```
This launches the terminal interface. Use your keyboard to navigate.

### 🌐 Web Mode (WASM)
To launch the web interface, you first need to compile the frontend assets, then run the backend server.

1. **Build the Frontend:**
   ```bash
   cd frontend
   trunk build --release
   cd ..
   ```

2. **Run the Server:**
   ```bash
   cargo run --release -- --web
   ```

3. **Open the App:**
   Navigate to [http://0.0.0.0:3000](http://0.0.0.0:3000) in your browser.

## ⌨️ Controls

### Web Interface
The web interface features a **Button-less "Enter-Only" Workflow**:
- **Type Answer**: Just start typing. The input box autofocuses.
- **Submit**: Press **Enter**.
- **Next Card**: Press **Enter** again.

### Terminal Interface
- **[Enter]**: Start Quiz / Submit Answer / Next Card.
- **[Esc]** or **`q`**: Quit.

## 🧠 Spaced Repetition System (SRS)
Both modes share the same SQLite database (`kana.db`). The app uses a simplified SM2 algorithm to schedule reviews, ensuring you focus on the characters you struggle with.
