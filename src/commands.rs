use crate::{bot::MyBot, osu_api::UserScore, osu_api::RecentScoreResponse};
use std::error::Error;
use crate::charts::{Chart, ChartQuery};

pub async fn handle_command(bot: &mut MyBot, sender: &str,target: &str, msg: &str, prefix: Option<String>) -> Result<(), Box<dyn Error>> {
    let mut split = msg.splitn(2, char::is_whitespace); // 只分割一次
    let mut command = split.next().unwrap_or("").to_lowercase();
    let raw_args = split.next().unwrap_or("").trim();
    command = command.replace("！", "!");
    let irc_name = prefix.unwrap_or_default();
    match command.as_str() {
        "!hello" => {
            let response = format!("Hello, {}!", &irc_name);
            bot.send_message(target, &response).await?;
        }
        "!info" | "!i" => {
            bot.send_beatmap_info().await?;
        }
        "!pick"=> {
            if sender == bot.room_host{
                handle_pick(bot, target,raw_args).await?;
            }
            else { 
                bot.send_message(target,"只有房主才能选歌哦").await?;
            }
        }
        "!abort" => {
            bot.vote_abort(&irc_name).await?;
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
        "!help" | "!h" => {
            bot.send_menu().await?;
        }
        "!about" => {
            bot.send_about().await?;
        }
        "!pr" | "!p" => {
            handle_recent_score(bot, target, &irc_name, false).await?;
        }
        "!re" | "!r" => {
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
async fn handle_pick(bot: &mut MyBot, target: &str,parms:&str) -> Result<(), Box<dyn Error>> {

    let query = match ChartQuery::parse(&parms.to_uppercase()) {
        Ok(q) => q,
        Err(e) => {
            bot.send_message(target, "输入的参数有误,请检查").await?;
            return Ok(());
        }
    };

    if let Some(chart) = bot.chart_db.query_with_fallback(&query)? {
        // println!("查询结果: {}", serde_json::to_string_pretty(&chart)?);
        bot.set_map(chart.chart_id).await?;
        let formatted_pick = format_pick(chart);
        bot.send_message(target, &formatted_pick).await?;
    } else {
        bot.send_message(target, "没有找到匹配的谱面").await?;
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
            // 这里就要算pp了
        }
        Ok(None) => {
            let message = if include_fails {
                format!("没有找到{}最近的成绩", username)
            } else {
                format!("没有找到{}最近pass的成绩", username)
            };
            bot.send_message(target, &message).await?;
        }
        Err(e) => {
            bot.send_message(target, &format!("获取最近成绩失败: {}", e)).await?;
        }
    }
    Ok(())
}

fn format_pick(chart_info:Chart) -> String {

    format!(
        "当前谱面来自: {} {} {}({}) {}{}",
        chart_info.competition_name.unwrap_or_default(),
        chart_info.season.unwrap_or_default(),
        chart_info.pool_name.unwrap_or_default(),
        chart_info.pool_index.unwrap_or_default(),
        chart_info.chart_type.unwrap_or_default(),
        chart_info.chart_type_index.unwrap_or_default(),
    )
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
