# bili-maestro

Bilibili 直播弹幕点歌/切视频控制器。实时读取弹幕，解析指令，通过 HTTP + WebSocket API 供外部程序消费。

## 功能

- 连接 Bilibili 直播间，实时接收
- 解析弹幕中的指令（点歌、切歌、切视频）
- 提供 HTTP REST API 推送接口
- 外部程序（播放器、OBS 插件等）可通过 API 获取指令
