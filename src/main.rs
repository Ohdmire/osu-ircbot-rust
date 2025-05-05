mod bot;
mod commands;
mod config;
mod pp_calculator;
mod osu_api;
mod events;

use bot::MyBot;
use config::get_config;
use std::env;
use dotenv::dotenv;

// bot设置
pub struct BotSettings {
    pub room_name:String,
    pub room_password:String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 .env 文件
    dotenv().ok();

    let config = get_config()?;
    
    let botsettings = BotSettings{
        room_name: env::var("ROOM_NAME").expect("ROOM_NAME must be set in .env file"),
        room_password: env::var("ROOM_PASSWORD").expect("ROOM_PASSWORD must be set in .env file"),
    };    
    let client_id = env::var("OSU_CLIENT_ID").expect("OSU_CLIENT_ID must be set in .env file");
    let client_secret = env::var("OSU_CLIENT_SECRET").expect("OSU_CLIENT_SECRET must be set in .env file");
    
    let mut bot = MyBot::new(config, client_id, client_secret,botsettings).await?;
    bot.run().await?;
    Ok(())
}
