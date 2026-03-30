//! LLM Gateway 统计 CLI
//!
//! 交互式 REPL，用于查询 LLM Gateway 统计数据

use clap::Parser;
use llm_gateway_statistics::cli::ReplApp;

/// LLM Gateway 统计 CLI 参数
#[derive(Parser, Debug)]
#[command(name = "llm-stats")]
#[command(author = "LLM Gateway Team")]
#[command(version = "0.1.0")]
#[command(about = "Interactive CLI for querying LLM Gateway statistics", long_about = None)]
struct Args {
    /// SQLite 数据库文件路径
    #[arg(short, long, default_value = "./stats.db")]
    db: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let mut app = ReplApp::new(&args.db).await?;
    app.run().await?;

    Ok(())
}
