use crate::bot::MyBot;
use std::error::Error;
use std::time::Instant;
use tokio::sync::Mutex;

pub async fn handle_command(bot: &mut MyBot, target: &str, msg: &str, prefix: Option<String>) -> Result<(), Box<dyn Error>> {
    let command = msg.split_whitespace().next().unwrap_or("");
    match command {
        "!hello" => {
            let response = format!("Hello, {}!", prefix.unwrap_or_default());
            bot.send_message(target, &response).await?;
        }
        "!create" => {
            create_room(bot).await?;
        }
        "!start" => {
            start_game(bot).await?;
        }
        "!abort" => {
            abort_game(bot).await?;
        }
        "!host" => {
            if let Some(new_host) = msg.split_whitespace().nth(1) {
                change_host(bot, new_host).await?;
            }
        }
        "!queue" | "!q" => {
            show_queue(bot, target).await?;
        }
        "!skip" => {
            vote_skip(bot, target, prefix).await?;
        }
        "!close" => {
            vote_close(bot, target, prefix).await?;
        }
        "!help" => {
            send_help(bot, target).await?;
        }
        "!pp" => {
            calculate_pp(bot, target, msg).await?;
        }
        _ => {}
    }
    Ok(())
}

async fn create_room(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    bot.send_message("BanchoBot", "!mp make ATRI1024's room").await?;
    println!("Sent room creation request to BanchoBot");
    Ok(())
}

async fn start_game(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    let current_room_id = *bot.room_id.lock().await;
    bot.send_message(&format!("#mp_{}", current_room_id), "!mp start").await?;
    bot.game_start_time = Some(Instant::now());
    Ok(())
}

async fn abort_game(bot: &mut MyBot) -> Result<(), Box<dyn Error>> {
    let current_room_id = *bot.room_id.lock().await;
    bot.send_message(&format!("#mp_{}", current_room_id), "!mp abort").await?;
    bot.game_start_time = None;
    Ok(())
}

async fn change_host(bot: &mut MyBot, new_host: &str) -> Result<(), Box<dyn Error>> {
    let current_room_id = *bot.room_id.lock().await;
    bot.send_message(&format!("#mp_{}", current_room_id), &format!("!mp host {}", new_host)).await?;
    bot.room_host = new_host.to_string();
    Ok(())
}

async fn show_queue(bot: &MyBot, target: &str) -> Result<(), Box<dyn Error>> {
    let queue = bot.room_host_list.join(" -> ");
    bot.send_message(target, &format!("Current queue: {}", queue)).await?;
    Ok(())
}

async fn vote_skip(bot: &mut MyBot, target: &str, prefix: Option<String>) -> Result<(), Box<dyn Error>> {
    if let Some(voter) = prefix {
        bot.approved_host_rotate_list.push(voter);
        let votes_needed = (bot.player_list.len() + 1) / 2;
        if bot.approved_host_rotate_list.len() >= votes_needed {
            bot.rotate_host().await?;
            bot.approved_host_rotate_list.clear();
            bot.send_message(target, "Host skipped due to vote.").await?;
        } else {
            bot.send_message(target, &format!("Vote to skip host: {} / {} votes", bot.approved_host_rotate_list.len(), votes_needed)).await?;
        }
    }
    Ok(())
}

async fn vote_close(bot: &mut MyBot, target: &str, prefix: Option<String>) -> Result<(), Box<dyn Error>> {
    if let Some(voter) = prefix {
        bot.approved_close_list.push(voter);
        if bot.approved_close_list.len() == bot.player_list.len() {
            let current_room_id = *bot.room_id.lock().await;
            bot.send_message(&format!("#mp_{}", current_room_id), "!mp close").await?;
            bot.send_message(target, "Room closed due to unanimous vote.").await?;
        } else {
            bot.send_message(target, &format!("Vote to close room: {} / {} votes", bot.approved_close_list.len(), bot.player_list.len())).await?;
        }
    }
    Ok(())
}

async fn send_help(bot: &MyBot, target: &str) -> Result<(), Box<dyn Error>> {
    let help_message = "Available commands: !hello, !create, !start, !abort, !host <player>, !queue (!q), !skip, !close, !help, !pp";
    bot.send_message(target, help_message).await?;
    Ok(())
}

async fn calculate_pp(bot: &mut MyBot, target: &str, msg: &str) -> Result<(), Box<dyn Error>> {
    let parts: Vec<&str> = msg.split_whitespace().collect();
    if parts.len() < 2 {
        bot.send_message(target, "Usage: !pp <accuracy> [mods]").await?;
        return Ok(());
    }

    let accuracy: f64 = parts[1].parse()?;
    let mods = if parts.len() > 2 { parse_mods(parts[2]) } else { 0 };

    let (stars, pp, max_pp) = bot.calculate_pp(mods, 0, accuracy)?;
    let response = format!("Stars: {:.2} | PP: {:.2}/{:.2} for {:.2}% accuracy", stars, pp, max_pp, accuracy);
    bot.send_message(target, &response).await?;

    Ok(())
}

fn parse_mods(mods_str: &str) -> u32 {
    // This is a very basic implementation. You might want to expand this.
    match mods_str.to_uppercase().as_str() {
        "HD" => 8,
        "HR" => 16,
        "DT" => 64,
        "HDHR" => 24,
        "HDDT" => 72,
        _ => 0,
    }
}
