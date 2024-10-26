use crate::{bot::MyBot, osu_api::UserScore, osu_api::RecentScoreResponse};
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
        "!pr" => {
            handle_recent_score(bot, target, &irc_name, false).await?;
        }
        "!re" => {
            handle_recent_score(bot, target, &irc_name, true).await?;
        }
        "!s" => {
            let user_id = bot.get_user_mut(&irc_name).await.unwrap().id.clone();
            let username = bot.get_user_mut(&irc_name).await.unwrap().username.clone();
            let beatmap_id = bot.beatmap_id;

            match bot.osu_api.get_user_score(user_id, beatmap_id).await {
                Ok(userscore) => {
                    let formatted_score = format_user_score(&username, &userscore, bot);
                    bot.send_message(target, &formatted_score).await?;
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

async fn handle_recent_score(bot: &mut MyBot, target: &str, irc_name: &str, include_fails: bool) -> Result<(), Box<dyn Error>> {
    let user_id = bot.get_user_mut(irc_name).await.unwrap().id.clone();
    let username = bot.get_user_mut(irc_name).await.unwrap().username.clone();

    match bot.osu_api.get_user_recent_score(user_id, include_fails).await {
        Ok(Some(score)) => {
            let formatted_score = format_score(&username, &score);
            bot.send_message(target, &formatted_score).await?;
        }
        Ok(None) => {
            let message = if include_fails {
                format!("No recent scores found for {}.", username)
            } else {
                format!("No recent pass scores found for {}.", username)
            };
            bot.send_message(target, &message).await?;
        }
        Err(e) => {
            bot.send_message(target, &format!("Failed to get recent score: {}", e)).await?;
        }
    }
    Ok(())
}

fn format_score(username: &str, score: &RecentScoreResponse) -> String {
    format!(
        "{}| [{} {} - {}]| {:.2}*| {}| [{}] {:.2}pp Acc: {:.2}% Combo: {}x| {}/{}/{}/{}| {}",
        username,
        score.format_url(),
        score.beatmapset.title_unicode,
        score.beatmapset.artist_unicode,
        score.beatmap.difficulty_rating,
        score.mods.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(""),
        score.rank,
        score.pp.unwrap_or(0.0),
        score.accuracy * 100.0,
        score.max_combo,
        score.statistics.count_300,
        score.statistics.count_100,
        score.statistics.count_50,
        score.statistics.count_miss,
        score.format_date()
    )
}

fn format_user_score(username: &str, score: &UserScore, bot: &MyBot) -> String {
    format!(
        "{}| [{} {} - {}]| {:.2}*| {}| [{}] {:.2}pp Acc: {:.2}% Combo: {}x| {}/{}/{}/{}| {}",
        username,
        score.score.format_url(bot.beatmap_id),
        bot.beatmap_title_unicode,
        bot.beatmap_artist_unicode,
        bot.beatmap_difficulty_rating,
        score.score.mods.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(""),
        score.score.rank,
        score.score.pp.unwrap_or(0.0),
        score.score.accuracy * 100.0,
        score.score.max_combo,
        score.score.statistics.count_300,
        score.score.statistics.count_100,
        score.score.statistics.count_50,
        score.score.statistics.count_miss,
        score.score.format_date()
    )
}
