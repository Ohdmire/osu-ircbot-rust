# osu-ircbot-rust

一个用 Rust 编写的 osu! irc 机器人。

## 简介

这个机器人连接到 osu! irc，并提供各种命令和功能。

## 构建和运行

1. 确保你已经安装了 Rust 和 Cargo。
2. 克隆这个仓库。
3. 创建一个 `.env` 文件，并填写以下配置：
   ```
   OSU_CLIENT_ID=203xx
   OSU_CLIENT_SECRET=4xxxxxxx
   IRC_NICKNAME=ATRI1024
   IRC_PASSWORD=b4axxxxx
   IRC_SERVER=irc.ppy.sh
   IRC_PORT=6667
   ROOM_NAME=ATRI1024高性能mp房测试ver0.9
   ROOM_PASSWORD=123
   ```
4. 运行 `cargo build --release` 来构建项目。
5. 运行 `target/release/osu-ircbot-rust` 来启动机器人。

## License

GPL-3.0 license