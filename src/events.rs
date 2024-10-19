use crate::bot::MyBot;
use std::error::Error;
use regex::Regex;
use crate::pp_calculator::PPCalculator;
use std::path::Path;


pub async fn handle_event(bot: &mut MyBot, target: &str, msg: &str) -> Result<(), Box<dyn Error>> {
    match (target, msg) {
        ("ATRI1024", m) if m.contains("Created the tournament match") => {
            parse_room_id(bot, m).await?;
            bot.join_channel(&format!("#mp_{}", *bot.room_id.lock().await)).await?;
            bot.set_room_password(bot.room_password.clone()).await?;
        }
        (_, m) if m.contains("Beatmap changed to") => {
            handle_beatmap_change(bot, m).await?;
        }
        (_, m) if m.contains("The match has started") => {
            handle_match_start(bot).await?;
        }
        (_, m) if m.contains("The match has finished") => {
            handle_match_finish(bot).await?;
        }
        (_, m) if m.contains("joined in slot") => {
            handle_player_join(bot, m).await?;
        }
        (_, m) if m.contains("left the game") => {
            handle_player_leave(bot, m).await?;
        }
        _ => {}
    }
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
            let (stars, pp, max_pp, pp_95_fc, pp_96_fc, pp_97_fc, pp_98_fc, pp_99_fc) = bot.pp_calculator.calculate_beatmap_details(mods)?;

            let pp_info = format!("Stars: {:.2} | 95%: {:.2}pp | 98%: {:.2}pp | 100%: {:.2}pp | Max: {:.2}pp", 
                                  stars, pp_95_fc, pp_98_fc, max_pp, max_pp);
            
            let beatmap_info = format!("Beatmap: {} [{}] | Length: {}s | Status: {}", 
                                       beatmap.beatmapset_id, beatmap.version, beatmap.total_length, beatmap.status);
            
            bot.send_message(&format!("#mp_{}", *bot.room_id.lock().await), &beatmap_info).await?;
            bot.send_message(&format!("#mp_{}", *bot.room_id.lock().await), &pp_info).await?;
        }
    }
    Ok(())
}

async fn handle_match_start(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.game_start_time = Some(std::time::Instant::now());
    println!("Match started");
    Ok(())
}

async fn handle_match_finish(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.game_start_time = None;
    println!("Match finished");
    bot.rotate_host().await?;
    Ok(())
}

async fn handle_player_join(bot: &mut MyBot, msg: &str) -> Result<(), Box<dyn Error>> {
    let re = Regex::new(r"(.+) joined in slot \d+")?;
    if let Some(captures) = re.captures(msg) {
        if let Some(name) = captures.get(1) {
            let player_name = name.as_str().to_string();
            bot.add_player(player_name.clone());
            println!("Player joined: {}", player_name);
            // 检查玩家是不是房间里面的第一个加入的
            if bot.player_list.len() == 1 {
                // 如果之前为空，将当前玩家设为主机
                bot.set_host(&player_name).await?;
                println!("Set {} as host (first player)", player_name);
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
            println!("Player left: {}", name.as_str());
            println!("Player list: {:?}", bot.player_list);
        }
    }
    Ok(())
}