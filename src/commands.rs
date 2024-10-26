use crate::bot::MyBot;
use std::error::Error;

pub async fn handle_command(bot: &mut MyBot, target: &str, msg: &str, prefix: Option<String>) -> Result<(), Box<dyn Error>> {
    let command = msg.split_whitespace().next().unwrap_or("");
    let irc_name = prefix.unwrap_or_default();
    match command {
        "!hello" => {
            let response = format!("Hello, {}!", &irc_name);
            bot.send_message(target, &response).await?;
        }
        "!info" | "!i" => {
            bot.send_beatmap_info().await?;
        }
        "!abort" => {
            bot.abort_game().await?;
        }
        "!queue" | "!q" => {
            bot.send_queue().await?;
        }
        "!skip" => {
            bot.vote_skip(&irc_name).await?;
        }
        "!close" => {
            bot.vote_close(&irc_name).await?;
        }
        "!start" => {
            bot.vote_start(&irc_name).await?;
        }
        "!ttl" => {
            bot.calculate_total_time_left().await?;
        }
        "!help" => {
            todo!()
        }
        "!pp" => {
            todo!()
        }
        "!s" => {
            // 先获取用户 ID
            let user_id = bot.get_user_mut(&irc_name).await.unwrap().id.clone();
            // 获取username
            let username = bot.get_user_mut(&irc_name).await.unwrap().username.clone();
            let beatmap_id = bot.beatmap_id;

            // 然后获取用户分数
            match bot.osu_api.get_user_score(user_id, beatmap_id).await {
                Ok(score) => {
                    bot.send_message(target, &format!("Score for {}: {:?}", username, score)).await?;
                },
                Err(e) => {
                    bot.send_message(target, &format!("Failed to get user score: {}", e)).await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
