use std::{env, fs};

use llm_gateway_config::GatewayConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = env::args().nth(1);
    let config = fs::read_to_string(config.as_deref().unwrap_or("config.toml"))?;
    let _config: GatewayConfig = config.parse()?;
    Ok(())
}
