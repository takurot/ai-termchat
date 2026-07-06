要点

* ベースは termchat の「LAN内自動発見 + サーバ不要 + TUI」を活かし、会話モデルだけを「人間A・人間B・AI」の常設3者会話に置き換えるのが最も筋が良いです。termchat は起動時に multicast でピア発見し、その後 TCP 接続を張る構造で、TUI は `tui-rs`、通信は `message-io` を使っています。さらに `?send` によるファイル送信も既にあります。 ([GitHub][1])
* Claude Code 連携は、v1 では「チャットアプリが Claude Code の skill を自前解釈する」のではなく、「Claude Code sidecar に仕事を委譲し、`.claude/skills/.../SKILL.md` をそのまま使う」方式が安全です。Claude Code の skills は `SKILL.md` を入口にし、YAML frontmatter で自動起動可否や許可ツールを制御でき、MCP・hooks・subagents も組み合わせられます。 ([Claude][2])
* Claude Code の channels は「実行中セッションへ外部イベントを流し込む」方向には魅力がありますが、現時点では research preview なので、本仕様では採用をオプション扱いにします。 ([Claude][3])

# termchat ベース 3者会話型 AI CLI チャットツール 機能仕様書

版: v0.1
仮称: triadchat

> **⚠️ 歴史文書:** これは初期構想ドキュメント（v0.1）です。現在の実装の真実は `docs/SPEC.md` を、設定の正は `src/config.rs`（および本ファイル §12）を参照してください。本ファイル内の技術選択の記述（例: `tui-rs`）は構想時点のもので、実装では `ratatui` に移行済みです。

# 1. 目的

同一ネットワーク内で、軽量な CLI/TUI ベースのチャットを提供する。
ただし本ツールは従来の 1 対 1 メッセンジャーではなく、すべての会話を「人間A・人間B・AI」の3者会話として扱う。

AI は雑談相手ではなく、次の役割を持つ常設参加者とする。

* clerk: 決定事項、TODO、要約の抽出
* moderator: 曖昧さの確認、論点整理
* operator: skills / MCP / shell / local tools の実行補助

本ツールの本質は「チャットツール」ではなく、
会話をそのまま構造化・実行可能にする協働端末である。

# 2. ベース前提

本仕様は termchat の以下の特性を前提にする。

* サーバ不要の分散型 LAN チャット
* 起動時 multicast によるピア探索
* 発見後は TCP 接続
* TUI ベース
* ファイル送信機能あり
* 既存の動画ストリーム機能は本仕様では採用しない

この前提により、ネットワーク層は大きく変えず、会話モデル・AI 層・スキル実行層を追加する方針とする。 ([GitHub][1])

# 3. プロダクトコンセプト

# 3.1 コア定義

会話の基本単位は DM ではなく Triad Room とする。

Triad Room =

* human_1
* human_2
* ai_agent_1

AI はルーム生成時に必ず 1 体アタッチされる。
AI はルームから外せないが、発言頻度は制御できる。

# 3.2 主要価値

* 人間同士の会話のズレをその場で補正する
* 決定事項と作業依頼をその場で構造化する
* Claude Code skills などを呼び出して、会話から実行に接続する
* CLI/TUI 上でアスキーアートアバターにより「誰が」「どの状態か」を即座に識別できる

# 3.3 非目標

* ビデオ会議の代替
* Slack/Teams の完全置換
* WAN 前提の大規模メッセージ基盤
* AI の完全自律実行を標準動作にすること

# 4. 想定ユースケース

# 4.1 開発レビュー

人間A:
この関数、責務が多すぎる気がする

人間B:
じゃあ認証周りを切り出したい

AI:
決定候補:

* auth ロジックを service 層へ分離
* 今回は public interface を維持

AI:
必要なら `/review-auth` を実行して既存 skill でレビューできます

# 4.2 現場運用 / ロボット

人間A:
amr-03 の停止原因を見たい

人間B:
昨日から充電待ちが多い

AI:
確認項目:

* 経路ブロック
* 充電待ち
* 通信断

AI:
`/inspect-amr amr-03` を提案

# 4.3 ログ解析

人間A:
このログ送る

人間B:
エラー要約して

AI:
受信ファイル `error.log`
上位異常:

1. timeout
2. reconnect loop
3. queue overflow

# 5. システム構成

# 5.1 構成要素

1. triadchat core

* termchat fork
* ピア探索
* TCP セッション
* TUI 描画
* 会話ログ管理

2. AI mediator

* 発言分類
* 要約
* タスク抽出
* skill invocation 判定
* 応答生成

3. skill bridge

* Claude Code sidecar adapter
* generic agent adapter
* local script adapter

4. capability adapters

* shell
* local RAG
* file analyzer
* robot / ops API
* MCP proxy

# 5.2 推奨アーキテクチャ

```text
[triadchat TUI]
   │
   ├─ LAN discovery / TCP transport
   │
   ├─ Room engine
   │   └─ Human A / Human B / AI participant
   │
   ├─ AI mediator
   │   ├─ summarize
   │   ├─ clarify
   │   ├─ task extract
   │   └─ route
   │
   └─ Skill bridge
       ├─ Claude Code sidecar
       ├─ Local scripts
       ├─ MCP-backed tools
       └─ Workspace RAG
```

# 6. 主要機能要件

# 6.1 ピア検出

要件:

* 同一 LAN 上の triadchat ノードを自動発見する
* termchat 互換の multicast discovery を流用する
* 発見後に TCP 接続を確立する
* ユーザー名、ノード名、AI役割、アバター概要を交換する

受入条件:

* 同一セグメント内で起動後 3 秒以内に peer 一覧へ反映
* peer の状態が alive / idle / busy / disconnected で表示される

# 6.2 ルームモデル

要件:

* 新規会話開始時に必ず AI participant を付与
* デフォルトは 3者 room
* 将来の拡張として 3人以上 + AI も可能だが、v1 の最適化対象は 2 humans + 1 AI

ルーム生成例:

```bash
triadchat room create @takuro @tanaka --ai ops-ai
```

内部モデル:

```json
{
  "room_id": "room-001",
  "members": [
    {"id": "takuro", "type": "human"},
    {"id": "tanaka", "type": "human"},
    {"id": "ops-ai", "type": "ai", "mode": "clerk"}
  ]
}
```

# 6.3 AI 介入モード

AI は次の 3 モードを持つ。

1. listener

* 原則発言しない
* 明示呼び出し時のみ応答

2. clerk

* 決定事項、TODO、要約を抽出
* 会話の終端または一定間隔で短く出す

3. moderator

* 曖昧語、未解決点、矛盾を検出
* 必要時のみ介入

4. operator

* skill / MCP / shell / local API 実行候補を提示
* 実行前確認を標準とする

標準設定:

* 既定モードは clerk
* room ごとに切替可能
* 発言頻度は low / normal / high を指定可

# 6.4 AI 発言トリガー

AI は以下条件でのみ自動介入する。

* 明確な依頼表現を検出したとき
* 期限、担当、決定を含む文を検出したとき
* 重要な曖昧語を検出したとき
* ファイル受信後の解析が有効なとき
* `/ai` `/skill` `/run` で明示呼び出しされたとき

自動介入しない条件:

* 単純な雑談
* 連続発言中で人間同士のテンポを阻害する場合
* side effect を伴う skill が未承認の場合

# 6.5 会話要約・構造化

要件:

* 未読要約
* 決定事項抽出
* TODO 抽出
* 担当者抽出
* 期限抽出
* 会話タグ付け

コマンド例:

```bash
/summary
/decisions
/todos
/context
```

出力例:

```text
決定事項
- 認証処理を service 層に分離
- 既存 API は維持

TODO
- takuro: auth 抽出設計
- tanaka: 回帰確認
```

# 6.6 アスキーアートアバター

# 6.6.1 基本要件

* すべての参加者は ASCII avatar を持つ
* human と AI でスタイルを分ける
* 1行アイコン / 3行 / 5行 の 3 モード
* TUI 幅に応じて自動縮退

# 6.6.2 状態連動

AI avatar は state と mood で変化する。

state:

* idle
* listening
* thinking
* warning
* acting
* failed

例:

```text
idle      [@@ ]
thinking  [.. ]
warning   [!! ]
acting    [>> ]
failed    [xx ]
```

ロボット系 AI 例:

```text
  .----.
 [| .. |]
  | -- |
 /|____|\
```

# 6.6.3 avatar 編集

コマンド例:

```bash
/avatar set @ops-ai robot_guardian
/avatar preview
/avatar generate "四角い顔の無口な監視ロボット"
```

v1 では手動設定を必須とし、生成は実験機能とする。

# 6.7 ファイル送信と AI 解析

termchat 由来のファイル送信機能を継承し、受信後に AI 解析を追加する。termchat 自体にも `?send` によるファイル送信があります。 ([GitHub][1])

要件:

* ログファイル受信
* Markdown / code / JSON / CSV 解析
* 受信後に自動で

  * 要約
  * 主要エラー抽出
  * 重要差分抽出
  * 次の action 提案
    を行う

コマンド例:

```bash
/send ./error.log
/analyze last-file
```

# 6.8 skill 実行

# 6.8.1 基本方針

本ツールは skill を「会話から起動できる実行ユニット」として扱う。
ただし Claude Code skills の仕様そのものは再実装せず、v1 では Claude Code sidecar に委譲する。

理由:

* Claude Code の skills は `SKILL.md` + frontmatter + supporting files を持つ
* 自動起動可否、手動専用化、allowed-tools などの仕様が既にある
* skills は `.claude/skills/<name>/SKILL.md` 配置で project/user/plugin scope を持てる
* custom commands は skills に統合済みで、`/skill-name` で呼び出せる ([Claude][2])

# 6.8.2 skill 実行モード

1. suggest

* AI が skill 候補のみ提示
* 実行しない

2. confirm

* AI が skill 候補提示
* ユーザー承認後に実行

3. auto-safe

* read-only skill のみ自動実行

4. manual

* `/skill <name>` でのみ起動

# 6.8.3 コマンド

```bash
/skills
/skill review-auth
/skill inspect-amr amr-03
/skill summarize-log ./error.log
```

# 6.8.4 skill メタ情報表示

```text
skill: inspect-amr
scope: project
invoke: manual
tools: Read, Bash, MCP(amr)
risk: medium
```

# 6.9 Claude Code sidecar 連携

# 6.9.1 方針

triadchat は workspace ごとに Claude Code sidecar を任意接続できる。
triadchat は sidecar に「会話文脈」「実行対象 skill」「添付ファイルパス」を渡す。
sidecar 側の Claude Code が `.claude/skills`、`.claude/agents`、MCP、hooks を使って処理する。

Claude Code では、skills に加えて subagents も使え、subagent は独立した context、tool access、permission を持つ。重い調査は subagent に逃がす設計が適している。 ([Claude][4])

# 6.9.2 連携レベル

Level 1:

* `claude -p` などの一発実行
* 同期応答

Level 2:

* 長時間タスク用 sidecar session
* triadchat が task id を保持
* 結果を room に戻す

Level 3:

* 実行中 Claude Code session へのイベント注入
* channels 利用
* 研究機能扱い

Claude Code の channels は、MCP server から実行中セッションへメッセージや webhook を push できる一方、research preview であり、v2.1.80 以降・claude.ai login・allowlist 制約があります。そのため本仕様では Level 3 はオプションとします。 ([Claude][3])

# 6.9.3 推奨呼び出しフロー

```text
triadchat
  → AI mediator が skill 候補を選ぶ
  → Claude sidecar adapter に task 作成
  → Claude Code が relevant skill を自動ロード or /skill-name 実行
  → 結果を短い自然文 + 構造化 JSON で triadchat に返す
  → room に投稿
```

# 6.10 MCP 連携

Claude Code は MCP で外部データやツールへ接続でき、HTTP transport を含む複数方式でサーバ追加が可能です。plugin 経由で MCP を束ねることもできます。 ([Claude][5])

本ツールでは MCP を直接しゃべるのではなく、次のどちらかで使う。

1. Claude Code 側から利用

* triadchat → Claude Code sidecar → MCP

2. triadchat 直接 adapter

* 読み取り専用の軽量 MCP を triadchat 自身が呼ぶ

v1 推奨:

* 書き込み系は Claude Code 側に集約
* triadchat 側は peer / room / transcript の責務に集中

# 6.11 hooks 連携

Claude Code の hooks は shell command、HTTP endpoint、LLM prompt を lifecycle の特定タイミングで自動実行できます。 ([Claude][6])

本仕様での用途:

* skill 実行後に transcript を room に戻す
* file edit 後に formatter 実行
* commit 前に lint / test 実行
* danger command を block
* AI 応答を triadchat event に変換

# 6.12 会話からの実行提案

AI は自然文を次の 4 つに分類する。

* discuss
* decide
* task
* execute

execute 判定時のみ `/run` 候補を提示する。

例:

```text
人間A: 認証周りだけ調べたい
AI: 候補
1. /skill review-auth
2. /skill find-oauth-utils
3. /agent security-reviewer
```

# 7. TUI / CLI 仕様

# 7.1 画面レイアウト

```text
┌─────────────────────────────────────────────────────────────┐
│ peers                     room: takuro + tanaka + ops-ai   │
├──────────────┬───────────────────────────────┬─────────────┤
│ (^_^) takuro │ takuro: この停止、充電待ち?    │ [@@ ] ops-ai│
│ (-_-) tanaka │ tanaka: その可能性高い         │ mode: clerk │
│ [@@ ] ops-ai │ ops-ai: 確認事項を整理します   │ state: think│
│              │ - 充電待ち                     │ skills: 12  │
│              │ - 経路ブロック                 │ mcp: amr,git│
├──────────────┴───────────────────────────────┴─────────────┤
│ > /skill inspect-amr amr-03                                 │
└─────────────────────────────────────────────────────────────┘
```

左:

* peer 一覧
* avatar
* presence

中央:

* 会話本体

右:

* AI ステータス
* room context
* skill 候補
* 直近 extracted tasks

# 7.2 主要コマンド

```bash
/help
/peers
/rooms
/room create @user1 @user2 --ai ops-ai
/room switch room-001

/ai mode clerk
/ai mode moderator
/ai quiet on
/ai quiet off

/summary
/todos
/decisions
/context

/skills
/skill <name> [args]
/run <proposal-id>
/cancel <task-id>

/avatar set <target> <preset>
/avatar preview
/avatar mode compact|normal|expressive

/send <file>
/analyze <file|last-file>
```

# 7.3 通知

通知イベント:

* peer joined
* peer left
* AI task started
* AI task finished
* approval needed
* skill failed
* action blocked

通知表現:

* status bar
* toast 1 行
* AI avatar state change

# 8. データモデル

# 8.1 message

```json
{
  "id": "msg-001",
  "room_id": "room-001",
  "sender_id": "ops-ai",
  "sender_type": "ai",
  "text": "確認事項を整理します",
  "kind": "chat",
  "timestamp": "2026-04-04T10:00:00+09:00",
  "intent": "clarify",
  "structured": {
    "todos": [],
    "decisions": [],
    "skills": ["inspect-amr"]
  }
}
```

# 8.2 room

```json
{
  "id": "room-001",
  "members": ["takuro", "tanaka", "ops-ai"],
  "ai_mode": "clerk",
  "topic": "amr-03 incident",
  "workspace": "/repo/project-a"
}
```

# 8.3 avatar

```json
{
  "target_id": "ops-ai",
  "preset": "robot_guardian",
  "style": "ascii",
  "size": "compact",
  "state": "thinking",
  "mood": "neutral"
}
```

# 8.4 skill registry view

```json
{
  "name": "inspect-amr",
  "backend": "claude_code",
  "path": ".claude/skills/inspect-amr/SKILL.md",
  "invoke_mode": "manual",
  "risk": "medium",
  "allowed_tools": ["Read", "Bash", "MCP"]
}
```

# 9. 権限・安全設計

# 9.1 権限レベル

1. read-only

* 要約
* 検索
* file read
* log parse

2. confirm-required

* shell
* git
* robot API write
* ticket update

3. blocked

* network exfiltration
* destructive bash
* untrusted skill auto-run

# 9.2 skill 安全ルール

* `disable-model-invocation: true` の skill は常に manual 扱い
* allowed-tools を UI に表示
* side effect を持つ skill は room 内の明示承認が必要
* 実行ログを transcript に残す
* result は room にサマリだけ返す
* raw output は展開時のみ表示

Claude Code の skills は frontmatter で model invocation と tools を制御できるため、本ツールでもその意味を UI に反映する。 ([Claude][2])

# 9.3 ネットワーク安全

* デフォルトは LAN 限定
* discovery address を設定可能
* peer fingerprint を初回保存
* trusted peer のみ skill execution 許可
* transcript はローカル保存を標準

# 10. 非機能要件

# 10.1 パフォーマンス

* 起動 1 秒以内
* peer discovery 3 秒以内
* TUI 描画 60fps は不要、体感遅延重視
* AI 非利用時は termchat 同等の軽さを維持

# 10.2 可搬性

* macOS
* Linux
* WSL
* Windows は後追い対応

# 10.3 可観測性

* local transcript
* skill execution log
* approval log
* error trace
* optional JSONL export

# 10.4 拡張性

* adapter interface で sidecar 追加可能
* avatar preset を追加可能
* MCP adapter を差し替え可能

# 11. 推奨ディレクトリ構成

```text
triadchat/
├── src/
├── avatars/
│   ├── humans/
│   └── ai/
├── skills/
│   └── adapters/
├── config/
│   └── default.toml
└── docs/

workspace/
├── .claude/
│   ├── skills/
│   │   ├── inspect-amr/
│   │   │   ├── SKILL.md
│   │   │   ├── reference.md
│   │   │   └── scripts/
│   │   └── review-auth/
│   │       └── SKILL.md
│   ├── agents/
│   │   └── security-reviewer.md
│   └── settings.local.json
```

Claude Code では project-level の skills は `.claude/skills/.../SKILL.md`、subagents は `.claude/agents/` に置けます。skills は supporting files を持て、subagents は独立 context と tool 制御を持ちます。 ([Claude][2])

# 12. 設定例

> 現在の正味の設定形状は `src/config.rs` の `Config` 構造体が真。以下は実装に合わせた例（`[network]`/`[ui]`/`[claude_code]` セクションは存在しない）。

```toml
# フラットキー (CLI フラグで上書き可能)
discovery_addr   = "238.255.0.1:5877"   # termchat 由来のデフォルト mcast
tcp_server_port  = 0                    # 0 = ランダム
user_name        = "takuro"
terminal_bell    = true

[language]
ai_output = "ja"   # ja | en | zh | ko
ui        = "ja"   # ja | en

[ai]
enabled      = true
provider     = "claude"   # claude | codex | gemini | custom
# command    = "/path/to/claude"
timeout_secs = 30

[security]
default_permission = "confirm-required"   # confirm-required | trusted-auto-safe | deny-remote-exec
trusted_peers      = ["takuro-mac", "tanaka-laptop"]

[user]
avatar    = "human_default"
ai_avatar = "ai_default"

# [theme] は初回起動時に自動生成される Color 定義一式。
```

# 13. MVP 範囲

# 13.1 MVP に含める

* termchat fork
* LAN discovery / TCP transport 維持
* 3者 room モデル
* ASCII avatar 表示
* AI clerk mode
* `/summary` `/todos` `/decisions`
* file send + log summary
* Claude Code sidecar 連携
* `/skills` `/skill <name>`
* 実行前確認 UI

# 13.2 MVP では除外

* 自動 avatar 生成
* channels ベース live injection
* agent teams 統合
* WAN relay
* 音声 / 動画
* 自動実行の高度化
* enterprise RBAC

# 14. 将来拡張

# 14.1 subagent 連携

Claude Code subagents は独立 context と tool access を持つため、triadchat 上では「AI の内部 worker」として扱える。大規模調査だけ subagent に逃がし、room には要約だけ返す設計がよい。 ([Claude][4])

# 14.2 agent teams 連携

Claude Code には agent teams もあり、チームメンバー間の直接メッセージや shared task list を扱えるが、experimental で coordination overhead も高い。よって triadchat v1 の標準 backend にはせず、研究機能扱いとする。 ([Claude][7])

# 14.3 plugin 化

将来的には triadchat 用 plugin を作り、skills / hooks / MCP server を一括配布可能にする。Claude Code 側でも plugin で skills・hooks・MCP を束ねられるため、配布運用の整合が取りやすい。 ([Claude][5])

# 15. 受け入れ基準

* 2 台の端末で起動し、peer discovery が成立する
* room 作成時に AI participant が自動付与される
* AI clerk が会話から TODO を抽出できる
* ASCII avatar が state に応じて変化する
* `.claude/skills/.../SKILL.md` を持つ workspace で `/skills` 一覧が見える
* `/skill review-auth` 実行で Claude Code sidecar 経由の結果が room に戻る
* side effect skill は confirm-required で止まる
* transcript に human / AI / skill result が時系列保存される

# 16. 実装方針の結論

最初に作るべきものは、次の 4 本柱です。

1. termchat fork による LAN/TUI 維持
2. 3者会話 room engine
3. ASCII avatar と AI state 表示
4. Claude Code sidecar + skills bridge

これで「軽量 CLI LAN チャット」から、
「AI を含む協働オペレーション端末」へ自然に進化できます。

次にやるべき実務的な落とし込みは、これをそのまま `SPEC.md` と `tasks.md` に分割する作業です。

[1]: https://github.com/lemunozm/termchat "GitHub - lemunozm/termchat: Terminal chat through the LAN with video streaming and file transfer. · GitHub"
[2]: https://code.claude.com/docs/en/skills "Extend Claude with skills - Claude Code Docs"
[3]: https://code.claude.com/docs/en/channels "Push events into a running session with channels - Claude Code Docs"
[4]: https://code.claude.com/docs/en/sub-agents "Create custom subagents - Claude Code Docs"
[5]: https://code.claude.com/docs/en/mcp "Connect Claude Code to tools via MCP - Claude Code Docs"
[6]: https://code.claude.com/docs/en/hooks "Hooks reference - Claude Code Docs"
[7]: https://code.claude.com/docs/en/agent-teams "Orchestrate teams of Claude Code sessions - Claude Code Docs"

