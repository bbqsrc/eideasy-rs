#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eideasy::cli::run().await
}
