mod client;
mod types;
mod dispatch;
mod commands;
mod runtime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    runtime::run().await
}
