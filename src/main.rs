mod config;
mod download;
mod error;
mod kakao;
mod retry;
mod slicer;
mod tui;

#[tokio::main]
async fn main() {
    let _ = color_eyre::install();

    if let Err(error) = tui::run().await {
        eprintln!("Ошибка: {error}");
        std::process::exit(1);
    }
}
