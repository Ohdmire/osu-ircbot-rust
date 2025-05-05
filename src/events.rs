use crate::bot::MyBot;
use std::error::Error;
use regex::Regex;
use crate::pp_calculator::PPCalculator;
use std::path::Path;


pub async fn handle_event(bot: &mut MyBot, target: &str, msg: &str) -> Result<(), Box<dyn Error>> {
    match (target, msg) {
        ("ATRI1024", m) if m.contains("Created the tournament match") => {
            handle_create_room(bot, m).await?;
        }
        (_, m) if m.contains("Beatmap changed to") => {
            handle_beatmap_change(bot, m).await?;
        }
        (_, m) if m.contains("All players are ready") => {
            handle_match_ready(bot).await?;
        }
        (_, m) if m.contains("The match has started") => {
            handle_match_start(bot).await?;
        }
        (_, m) if m.contains("The match has finished") => {
            handle_match_finish(bot).await?;
        }
        (_, m) if m.contains("Aborted the match") => {
            handle_match_abort(bot).await?;
        }
        (_, m) if m.contains("joined in slot") => {
            handle_player_join(bot, m).await?;
        }
        (_, m) if m.contains("left the game") => {
            handle_player_leave(bot, m).await?;
        }
        (_, m) if m.starts_with("Slot") => {
            handle_slot(bot, m).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn handle_create_room(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    parse_room_id(bot, msg).await?;
    bot.join_channel(&format!("#mp_{}", *bot.room_id.lock().await)).await?;
    bot.set_room_password(bot.room_password.clone()).await?;
    bot.save_room_id_to_file().await?;
    Ok(())
}


async fn parse_room_id(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"https://osu\.ppy\.sh/mp/(\d+)")?;
    if let Some(captures) = re.captures(msg) {
        if let Some(id) = captures.get(1) {
            let new_room_id = id.as_str().parse::<u32>()?;
            {
                let mut room_id = bot.room_id.lock().await;
                *room_id = new_room_id;
            }
            println!("Room ID set to: {}", new_room_id);
        }
    }
    Ok(())
}

async fn handle_beatmap_change(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"Beatmap changed to: (.*) \((https://osu\.ppy\.sh/b/(\d+))\)")?;
    if let Some(captures) = re.captures(msg) {
        if let Some(id) = captures.get(3) {
            bot.beatmap_id = id.as_str().parse::<u32>()?;
            println!("Beatmap ID changed to: {}", bot.beatmap_id);
            
            // 获取谱面信息
            let beatmap = bot.osu_api.get_beatmap_info(bot.beatmap_id).await?;

            // 写入一些数据
            bot.beatmap_length = beatmap.total_length;
            bot.beatmap_difficulty_rating = beatmap.difficulty_rating;
            bot.beatmap_title_unicode = beatmap.beatmapset.title_unicode.clone();
            bot.beatmap_artist_unicode = beatmap.beatmapset.artist_unicode.clone();

            bot.beatmap_info = beatmap.get_formatted_info();
            
            bot.send_beatmap_info().await?;
            
            // 下载谱面
            bot.osu_api.download_beatmap(bot.beatmap_id).await?;

            // 更新 beatmap_path
            bot.beatmap_path = format!("./maps/{}.osu", bot.beatmap_id);
            
            // 检查文件是否存在
            if !Path::new(&bot.beatmap_path).exists() {
                println!("Beatmap file not found: {}", bot.beatmap_path);
                // 这里可以添加下载谱面的逻辑，或者发送一条消息说明谱面文件不存在
                bot.send_message(&format!("#mp_{}", *bot.room_id.lock().await), "Beatmap file not found. Unable to calculate PP.").await?;
                return Ok(());
            }

            // 如果文件存在，继续处理
            bot.pp_calculator = PPCalculator::new(bot.beatmap_path.clone());

            let mods = 0;
            let (stars, max_pp, pp_95_fc, pp_96_fc, pp_97_fc, pp_98_fc, pp_99_fc) = bot.pp_calculator.calculate_beatmap_details(mods)?;

            let beatmap_pp_info = format!("Stars: {:.2} | 95%: {:.2}pp | 96%: {:.2}pp | 97%: {:.2}pp | 98%: {:.2}pp | 99%: {:.2}pp | Max: {:.2}pp", 
                                  stars, pp_95_fc, pp_96_fc, pp_97_fc, pp_98_fc, pp_99_fc, max_pp);

            bot.beatmap_pp_info = beatmap_pp_info;

            bot.send_message(&format!("#mp_{}", *bot.room_id.lock().await), &bot.beatmap_pp_info).await?;
        }
    }
    Ok(())
}

async fn handle_slot(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"Slot \d+\s+(?:Not Ready|Ready)\s+https://osu\.ppy\.sh/u/\d+\s+(.+?)(?:\s+\[.+?\])?$")?;

    if let Some(captures) = re.captures(msg) {
        if let Some(player_name) = captures.get(1) {
            let player_name = player_name.as_str().trim().to_string();
            bot.add_player(player_name.clone());
            println!("Added player from slot: {}", player_name);
        }
    }
    
    Ok(())
}

async fn handle_match_ready(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.start_game().await?;
    Ok(())
}

async fn handle_match_start(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.beatmap_start_time = Some(std::time::Instant::now());
    bot.is_game_started = true;
    println!("Match started");
    Ok(())
}

async fn handle_match_finish(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.beatmap_end_time = Some(std::time::Instant::now());
    bot.is_game_started = false;
    println!("Match finished");
    // 比赛结束时，删除不在player_list中的玩家
    bot.remove_player_not_in_list();
    if is_fully_played(bot) {
        bot.rotate_host().await?;
    }
    bot.send_queue().await?;
    Ok(())
}

async fn handle_match_abort(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.beatmap_end_time = Some(std::time::Instant::now());
    bot.is_game_started = false;
    println!("Match aborted");
    // 比赛丢弃时时，删除不在player_list中的玩家
    bot.remove_player_not_in_list();
    if is_fully_played(bot) {
        bot.rotate_host().await?;
    }
    bot.send_queue().await?;
    Ok(())
}

fn is_fully_played(bot: &MyBot) -> bool {
    let played_len = bot.beatmap_end_time.unwrap_or_else(|| std::time::Instant::now()).duration_since(bot.beatmap_start_time.unwrap_or_else(|| std::time::Instant::now())).as_secs();
    println!("Played length: {}s ? {}s 1/2beatmap_length", played_len, bot.beatmap_length / 2);
    played_len >= bot.beatmap_length / 2
}

async fn handle_player_join(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"(.+) joined in slot \d+")?;
    if let Some(captures) = re.captures(msg) {
        if let Some(name) = captures.get(1) {
            let player_name = name.as_str().to_string();
            bot.add_player(player_name.clone());
            bot.send_welcome(player_name.clone()).await?;
            bot.save_latest_info_to_file().expect("无法写入bot state");
            println!("Player joined: {}", player_name);
            // 检查玩家是不是房间里面的第一个加入的
            if bot.player_list.len() == 1 {
                // 如果之前为空，将当前玩家设为主机
                bot.set_host(&player_name).await?;
                println!("Set {} as host (first player)", player_name);
                bot.set_free_mod().await?;
                println!("Set FreeMod");
            }
            println!("Player list: {:?}", bot.player_list);
        }
    }
    Ok(())
}

async fn handle_player_leave(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"(.+) left the game")?;
    if let Some(captures) = re.captures(msg) {
        if let Some(name) = captures.get(1) {
            bot.remove_player(name.as_str());
            bot.save_latest_info_to_file().expect("无法写入bot state");
            println!("Player left: {}", name.as_str());
            println!("Player list: {:?}", bot.player_list);
        }
    }
    Ok(())
}



