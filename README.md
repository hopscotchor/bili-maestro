# bili-maestro

Bilibili 直播弹幕点歌/切视频控制器。连接直播间 WebSocket，实时读取弹幕，解析指令，通过 HTTP + WebSocket API 供外部程序消费。

## 功能

- 连接 Bilibili 直播间，实时接收弹幕
- 解析弹幕中的指令（点歌、切歌、切视频）
- 提供 HTTP REST API 和 WebSocket 推送接口
- 外部程序（播放器、OBS 插件等）可通过 API 获取指令

## 使用

```bash
# 直接指定房间号
bili-maestro --room 12345

# 指定端口
bili-maestro --room 12345 --port 9090

# 使用配置文件
bili-maestro --config config.toml
```

## 弹幕指令

| 弹幕内容 | 指令 | 说明 |
|----------|------|------|
| `点歌 <歌名>` | SongRequest | 请求歌曲 |
| `切歌` | SkipSong | 跳过当前 |
| `切视频` / `下一个` | NextVideo | 下一个视频 |

指令关键词可在配置文件中自定义。

## API

### HTTP

```
GET  /api/health              # 健康检查
GET  /api/status              # 连接状态 + 在线人数
GET  /api/commands            # 待处理指令列表
GET  /api/commands?type=song  # 按类型过滤 (song/skip/next)
POST /api/commands/{id}/ack   # 确认指令已处理
GET  /api/danmaku?limit=50    # 最近弹幕
```

### WebSocket

```
ws://localhost:8080/ws/commands   # 实时推送指令
ws://localhost:8080/ws/danmaku    # 实时推送全部弹幕
```

### 消息格式

```json
{
  "id": "uuid",
  "timestamp": "2026-03-17T10:00:00Z",
  "user_uid": 12345,
  "username": "用户名",
  "command_type": { "type": "SongRequest", "content": "晴天" },
  "raw": "点歌 晴天"
}
```

### 外部程序集成

1. **HTTP 轮询**: 定期 `GET /api/commands`，处理后 `POST /api/commands/{id}/ack`
2. **WebSocket 订阅**: 连接 `ws://localhost:8080/ws/commands` 实时接收
3. **混合**: WS 实时接收 + HTTP 兜底

## 配置文件

参考 `config.example.toml`：

```toml
room_id = 12345
api_port = 8080

[commands]
song_prefix = "点歌 "
skip_keyword = "切歌"
next_keywords = ["切视频", "下一个"]
```

## 构建

```bash
cargo build --release
```

## License

MIT
