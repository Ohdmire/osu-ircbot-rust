mod bot;
mod commands;
mod config;
mod pp_calculator;
mod osu_api;

use bot::MyBot;
use config::get_config;
use std::env;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 .env 文件
    dotenv().ok();

    let config = get_config()?;
    
    let client_id = env::var("OSU_CLIENT_ID").expect("OSU_CLIENT_ID must be set in .env file");
    let client_secret = env::var("OSU_CLIENT_SECRET").expect("OSU_CLIENT_SECRET must be set in .env file");
    
    let mut bot = MyBot::new(config, client_id, client_secret).await?;
    bot.run().await?;
    Ok(())
}