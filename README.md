# osu-ircbot-rust

一个用 Rust 编写的 osu! irc 机器人。

## 简介

这个机器人连接到 osu! irc，并提供各种命令和功能。

```bash
!queue(!q) 查看队列 | 
!abort 投票丢弃游戏 | 
!start 投票开始游戏 | 
!skip 投票跳过房主 | 
!pr(!p) 查询最近pass成绩 | 
!re(!r) 查询最近成绩 | 
!s 查询当前谱面最好成绩| 
!info(!i) 返回当前谱面信息| 
!pick 挑选一张赛图 |
!ttl 查询剩余时间 | 
help(!h) 查看帮助 | 
!about 关于机器人 |
```
关于!pick

可挑选的指令如下以MP5比赛为例子
- 比赛名: MP5
- 赛季: S22
- 图池: -1(决赛)
- 模组: HD
- 模组顺序: 1


- `!pick MP5 S22-1 HD1`
- 谱面来自: MP5 S22 决赛 HD1

支持自动补完搜素,如有多个结果则随机一个

**下面的指令是合法的**
```bash
"!pick",
"!pick HD",
"!pick HD1",
"!pick MP5",
"!pick MP5 HD",
"!pick MP5 HD1",
"!pick MP5 S22",
"!pick MP5 S22 HD",
"!pick MP5 S22 HD1",
"!pick MP5 S22-1",
"!pick MP5 S22-1 HD"
"!pick MP5 S22-1 HD1"
```

> 当无法查找到结果时会默认按照下面顺序进行**回退**
> 
>也就是查询的条件的某个值会变为`None`
- 图池
- 模组顺序
- 赛季
> 如果上述fallback都失败则才会返回空
## 运行

1. 到`Release`页面下载最新版本
2. 确保创建了`.env`文件（格式在下文）
3. 双击运行即可

## 自行编译

1. 确保你已经安装了 Rust 和 Cargo。
2. 克隆这个仓库。
3. 创建一个 `.env` 文件，并填写以下配置：
   ```
   OSU_CLIENT_ID=203xx
   OSU_CLIENT_SECRET=4xxxxxxxxxxxx
   IRC_NICKNAME=ATRI1024
   IRC_PASSWORD=b4axxxxx
   IRC_SERVER=irc.ppy.sh
   IRC_PORT=6667
   ROOM_NAME="ATRI高性能mp房测试ver0.9"
   ROOM_PASSWORD=123
   ```
4. 运行 `cargo build --release` 来构建项目。
5. 运行 `target/release/irc_bot` 来启动机器人。

## License

GPL-3.0 license
