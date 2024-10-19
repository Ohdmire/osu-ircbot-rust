use crate::bot::MyBot;
use std::error::Error;

pub async fn handle_command(bot: &mut MyBot, target: &str, msg: &str, prefix: Option<String>) -> Result<(), Box<dyn Error>> {
    let command = msg.split_whitespace().next().unwrap_or("");
    match command {
        "!hello" => {
            let response = format!("Hello, {}!", prefix.unwrap_or_default());
            bot.send_message(target, &response).await?;
        }
        "!create" => {
            bot.create_room().await?;
        }
        "!start" => {
            bot.start_game().await?;
        }
        "!abort" => {
            bot.abort_game().await?;
        }
        "!queue" | "!q" => {
            bot.send_queue().await?;
        }
        "!skip" => {
            todo!()
        }
        "!close" => {
            todo!()
        }
        "!help" => {
            todo!()
        }
        "!pp" => {
            todo!()
        }
        _ => {}
    }
    Ok(())
}
