use irc::client::prelude::Config;
use std::env;

pub fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    
    let nickname = env::var("IRC_NICKNAME").expect("NICKNAME must be set");
    let password = env::var("IRC_PASSWORD").expect("IRC_PASSWORD must be set");

    Ok(Config {
        nickname: Some(nickname),
        server: Some("irc.ppy.sh".to_owned()),
        port: Some(6667),
        password: Some(password),
        use_tls: Some(false),
        ..Config::default()
    })
}