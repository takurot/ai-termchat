# ai-termchat (triadchat) 実装計画

最終更新: 2026-05-22
ベース仕様: [SPEC.md](./SPEC.md) v0.3

## ステータス凡例

| 記号 | 意味 |
|------|------|
| `📋 TODO` | 未着手 |
| `🔄 In Progress` | 作業中 |
| `✅ Done` | 完了 |
| `🚫 Blocked` | 依存 PR 待ち |

> 2026-05-22 時点のコードベースでは、PR-01〜PR-16相当のPhase 0/1/2実装は完了済み。
> 以降の更新はGitHub Issuesを優先し、このPLANは実装済みマイルストーンの参照として扱う。

---

## 依存グラフ

```
PR-01 (fork)
  └─► PR-02 (tokio)
        ├─► PR-03 (SidecarAdapter)
        │     └─► PR-04a (言語設定) ★
        │           └─► PR-04 (prompt.rs) ★
        │                 └─► PR-05 (AiMediator + trigger)
        │                       └─► PR-06 (summary コマンド)
        │                                 │
        │                                 └─► PR-13 (AvatarPlugin trait)
        │                                       └─► PR-14 (3ペイン TUI)
        │                                             └─► PR-15 (AvatarManager)
        │                                                   └─► PR-16 (/avatar cmd)
        │
        └─► PR-07 (NetMessage 拡張)
              └─► PR-08 (Room エンジン + LAN)
                    └─► PR-09 (SkillRegistry)
                          └─► PR-10 (SkillExecutor + 承認 UI)
                                ├─► PR-11 (/send 移植)
                                └─► PR-12 (Transcript)

━━━━━━━━━━━━━━━━━━━
Phase 0 完了: PR-06 マージ後
Phase 1 完了: PR-11, PR-12 マージ後
Phase 2 完了: PR-16 マージ後
```

---

## Phase 0 — PoC (1人+AI)

> 目標: シングルノードで `/summary` が動くこと
> 完了条件: 自分で打ち込んで TODO と決定事項が正確に出ること

---

### PR-01 — fork + リネーム + コマンドプレフィックス変更

**ステータス:** `✅ Done`
**依存:** なし
**ブランチ:** `feat/fork-rename`

**変更ファイル:**
```
Cargo.toml               name = "triadchat"
src/commands/mod.rs      COMMAND_PREFIX = "/"  (1行)
src/config.rs            config dir = "triadchat"
src/ui.rs                タイトル "LAN Room" → "triadchat"
README.md                プロジェクト説明更新
```

**やること:**
- [x] termchat v1.3.1 を fork してリポジトリ作成
- [x] `Cargo.toml` の `name`, `description`, `homepage`, `repository` を更新
- [x] `COMMAND_PREFIX` を `"?"` → `"/"` に変更
- [x] `Config::from_config_file()` のパスを `"termchat"` → `"triadchat"` に変更
- [x] video stream feature (`stream-video`) を Cargo.toml から除外
- [x] `cargo build` と既存 `cargo test` が通ることを確認

**リスク:** なし (機械的変更)

**受け入れ基準:**
- [x] `cargo build --release` が通る
- [x] `cargo test` が通る
- [x] 起動して既存チャット機能が動く (コマンドプレフィックスが `/` になっている)

---

### PR-02 — tokio Runtime 組み込み + Signal 拡張

**ステータス:** `✅ Done`
**依存:** PR-01
**ブランチ:** `feat/tokio-runtime`

**変更ファイル:**
```
Cargo.toml               tokio, anyhow, thiserror, tracing, serde_json 追加
src/application.rs       Runtime::new() + Handle 保持, Signal 拡張
src/util.rs              Error 型を anyhow::Error へ段階移行開始
```

**やること:**
- [x] `Cargo.toml` に依存追加:
  ```toml
  tokio      = { version = "1", features = ["full"] }
  anyhow     = "1"
  thiserror  = "1"
  tracing    = "0.1"
  tracing-subscriber = { version = "0.3", features = ["env-filter"] }
  serde_json = "1"
  ```
- [x] `Signal` に `AiResponse(String)` (暫定型) と `SkillDone` を追加
- [x] `Application` に `tokio::runtime::Runtime` フィールドを追加
- [x] `Application::new()` で `Runtime::new()` を生成、`handle()` を保持
- [x] **`#[tokio::main]` は使わない** (message-io スレッドとの競合回避)
- [x] ダミーの `handle.spawn(async { ... signal ... })` でループ内受信を確認するテストを書く

**リスク:** message-io の内部スレッドと tokio executor の競合
**対策:** `Runtime::new()` + `Handle::spawn()` で tokio を完全に分離

**受け入れ基準:**
- [x] `cargo build` が通る
- [x] 既存チャット機能が壊れていない
- [x] tokio spawn したタスクから `Signal::AiResponse` を送り、アプリループで受信できる (統合テスト)

---

### PR-03 — SidecarAdapter (claude -p + タイムアウト)

**ステータス:** `✅ Done`
**依存:** PR-02
**ブランチ:** `feat/sidecar-adapter`

**変更ファイル:**
```
Cargo.toml               which = "4" 追加
src/ai/mod.rs            新規
src/ai/sidecar.rs        新規
tests/sidecar_mock.rs    新規
```

**やること:**
- [x] `src/ai/sidecar.rs` を実装:
  - `SidecarAdapter::new(workspace)` — `which::which("claude")` で存在確認
  - `ask(&self, prompt: &str) -> anyhow::Result<String>` — 30秒タイムアウト
  - stdout が空なら `Err("empty response")`
  - stderr 非ゼロなら `Err` にメッセージを含める
  - プロンプト長上限: 50,000 文字 (超過時は先頭から切り詰め)
- [x] `Application::new()` で `SidecarAdapter::new()` を呼び、失敗したら警告表示 + `provider = disabled` にフォール
- [x] `tests/sidecar_mock.rs`: `claude` を `echo` に差し替えた mock で `ask()` が `Ok` を返すこと
- [x] タイムアウトテスト: `sleep 31` を実行して `Err` が返ること

**リスク:** `claude -p` の出力形式変化
**対策:** stdout を raw String で返し、パースは呼び出し元に委譲

**受け入れ基準:**
- [x] `claude` コマンドがない環境で起動時にわかりやすいエラーが出る
- [x] 30秒でタイムアウトして `Err` が返る (テスト)
- [x] mock で `ask()` が正常に `Ok(String)` を返す (テスト)

---

### PR-04a — 言語設定 (LanguageConfig + UI ローカライズ)

**ステータス:** `✅ Done`
**依存:** PR-03
**ブランチ:** `feat/language-config`

**変更ファイル:**
```
src/config.rs              LanguageConfig 追加、$LANG 自動判定
src/ai/prompt.rs           lang_instruction() ヘルパー追加 (この PR で骨格のみ)
src/ui/messages.rs         新規 (UI システムメッセージの ja/en 文字列定数)
```

**やること:**
- [x] `LanguageConfig` を `src/config.rs` に追加:
  ```rust
  pub struct LanguageConfig {
      pub ai_output: String,  // "ja" | "en" | "zh" | "ko"
      pub ui: String,         // "ja" | "en"
  }
  ```
- [x] `Default` 実装: `$LANG` 環境変数から自動判定、不明なら `"ja"`
- [x] `Config` に `pub language: LanguageConfig` を追加
- [x] `config.toml` のデフォルト生成に `[language]` セクションを追加
- [x] `src/ai/prompt.rs` に `lang_instruction(lang: &str) -> &'static str` を追加:
  - `"ja"` → `"必ず日本語で出力してください。"`
  - `"en"` → `"Respond in English."`
  - `"zh"` → `"请用中文回答。"`
  - `"ko"` → `"한국어로 답변해 주세요。"`
  - その他 → `"Respond in English."` (フォールバック)
- [x] `src/ui/messages.rs` に UI システムメッセージを定義:
  ```rust
  pub struct Messages {
      pub connected: &'static str,      // "is online" / "が接続しました"
      pub disconnected: &'static str,
      pub thinking: &'static str,       // "thinking..." / "考え中..."
      pub failed: &'static str,
      pub skill_confirm: &'static str,  // "[%s] 実行しますか? [y/n]"
      // ...
  }
  pub fn messages(lang: &str) -> &'static Messages { ... }
  ```
- [x] 既存のハードコードされた英語文字列を `messages()` に置き換え

**対応言語 (v1):**
- AI 出力: `ja` / `en` / `zh` / `ko`
- UI メッセージ: `ja` / `en`

**受け入れ基準:**
- [x] `config.toml` に `[language]` セクションが生成される
- [x] `ai_output = "en"` に設定すると AI が英語で応答する
- [x] `ui = "en"` に設定すると接続通知・エラーが英語になる
- [x] `$LANG=ja_JP.UTF-8` の環境でデフォルトが `"ja"` になる
- [x] `$LANG=en_US.UTF-8` の環境でデフォルトが `"en"` になる
- [x] 未知の言語コードを設定してもパニックしない (フォールバック)

---

### PR-04 — prompt.rs (プロンプト設計 + パーサー) ★ 最重要

**ステータス:** `✅ Done`
**依存:** PR-03
**ブランチ:** `feat/prompt-design`

**変更ファイル:**
```
src/ai/prompt.rs          新規 (プロンプトテンプレート)
src/ai/parser.rs          新規 (LLM 出力パーサー)
src/message.rs            AiPayload, StructuredOutput, TodoItem 型定義
tests/prompt_quality.rs   新規 (ゴールデンテスト)
tests/fixtures/           新規
  dev_review.txt
  ops_incident.txt
  simple_todo.txt
  ambiguous.txt
  no_todo.txt
  expected/
    dev_review.json
    ops_incident.json
    ...
```

**やること:**
- [x] `src/ai/prompt.rs` を実装:
  - `summary_prompt(transcript: &str) -> String`
  - `intervene_prompt(transcript: &str, last_messages: &[&str]) -> String`
  - `todos_prompt(transcript: &str) -> String`
  - 会話長上限: 直近 100 行に切り詰めるヘルパー `truncate_transcript()`
- [x] `src/ai/parser.rs` を実装:
  - `parse_structured_output(raw: &str) -> StructuredOutput`
  - `INTENT:` / `TEXT:` / `STRUCTURED:` 行を抽出
  - パース失敗時は `StructuredOutput::raw(raw)` として raw text を返す (パニックしない)
- [x] `AiPayload`, `StructuredOutput`, `TodoItem` を `src/message.rs` に定義
- [x] テスト用会話フィクスチャを `tests/fixtures/` に作成 (最低5パターン)
- [x] `tests/prompt_quality.rs` で期待出力との一致を検証
  - 担当者抽出の精度
  - 決定事項の正確な抽出
  - 雑談では SKIP が返ること

**リスク:** LLM 出力が毎回変わる
**対策:** フォーマット指示を明示。パーサーは defensive に。ゴールデンテストはフィクスチャ応答で検証

**受け入れ基準:**
- [x] 5パターンのフィクスチャで TODO/決定事項が正確に抽出される
- [x] パース失敗 (想定外フォーマット) でパニックしない
- [x] `truncate_transcript()` が 100 行超を正しく切り詰める

---

### PR-05 — AiMediator + 介入トリガー

**ステータス:** `✅ Done`
**依存:** PR-04
**ブランチ:** `feat/ai-mediator`

**変更ファイル:**
```
src/ai/mod.rs             AiMediator 実装
src/ai/trigger.rs         新規 (should_intervene)
src/ai/classifier.rs      新規 (MessageClass)
src/state.rs              ai_state, ai_mode, ai_thinking, abort_handle 追加
src/application.rs        process_ai_response(), process_terminal_event() 更新
tests/trigger_test.rs     新規
tests/classifier_test.rs  新規
```

**やること:**
- [x] `AiState`, `AiMode` を `src/state.rs` に追加
- [x] `AppState` に以下を追加:
  - `ai_state: AiState`
  - `ai_mode: AiMode`
  - `ai_thinking: bool`
  - `abort_handle: Option<tokio::task::AbortHandle>` (リーク防止)
  - `last_ai_at: Option<Instant>`
  - `human_streak: usize`
- [x] `src/ai/trigger.rs` — `should_intervene()` を実装
  - `ai_thinking == true` → false
  - cooldown (30秒) チェック
  - human streak (3連続) チェック
  - mode 別判定
- [x] `src/ai/classifier.rs` — `MessageClass` 分類を実装
- [x] `Application::process_terminal_event()` の Enter 処理に介入判定を追加
- [x] `Application::process_ai_response()` を実装:
  - `state.ai_thinking = false`
  - `state.ai_state = Idle`
  - `state.add_message(AiChatMessage)`
- [x] AI が thinking 中はチャットパネルのタイトルに `[ops-ai: thinking...]` を表示
- [x] `tests/trigger_test.rs` — 全条件のユニットテスト

**リスク:** `ai_thinking` のリーク (spawn panic 時)
**対策:** `AbortHandle` を `AppState` に保持、新しい spawn 前に前の handle を abort

**受け入れ基準:**
- [x] thinking 中にヘッダー表示が変わる
- [x] 二重呼び出しが発生しない (テスト)
- [x] `should_intervene()` の全条件がテストを通る

---

### PR-06 — /summary /todos /decisions /context コマンド (Phase 0 完了)

**ステータス:** `✅ Done`
**依存:** PR-05
**ブランチ:** `feat/summary-commands`

**変更ファイル:**
```
src/commands/mod.rs         CommandManager 更新
src/commands/ai_cmd.rs      新規 (/ai mode, /ai quiet, /ai freq)
src/commands/summary_cmd.rs 新規 (/summary, /todos, /decisions, /context)
src/ui/chat_panel.rs        AI メッセージを ✦ マーク付きで描画
```

**やること:**
- [x] `src/commands/summary_cmd.rs` を実装:
  - `/summary` — `summary_prompt()` → `sidecar.ask()` → チャットに表示
  - `/todos` — `todos_prompt()` → parse → TODO リスト表示
  - `/decisions` — 構造化出力から decisions を表示
  - `/context` — 直近 50 行の会話をそのままチャットに表示
  - すべて非同期: `handle.spawn(...)` → `Signal::AiResponse`
- [x] `src/commands/ai_cmd.rs` を実装:
  - `/ai mode <clerk|listener|moderator|operator>` — `state.ai_mode` 切替
  - `/ai quiet <on|off>` — `ai_mode = Listener` に切替
  - `/ai freq <low|normal|high>` — `trigger_config` 更新
- [x] AI メッセージを `✦` マーク + 別色で描画
- [x] `/help` に新コマンドを追加
- [x] Phase 0 受け入れ基準全項目を手動確認

**受け入れ基準 (Phase 0 完了ライン):**
- [x] `/summary` で要約が出る
- [x] `/todos` で担当者付き TODO が出る
- [x] `/decisions` で決定事項が出る
- [x] AI の自動介入が clerk mode で動く
- [x] 30秒タイムアウトで `[ops-ai: failed]` 表示
- [x] `/ai mode listener` で AI が黙る
- [x] `claude` なし環境で起動時エラーが出る

---

## Phase 1 — MVP (2人+AI、LAN)

> 目標: 2台で動く + 1 skill が非同期実行できること
> Phase 0 完了 (PR-06 マージ) 後に着手する

---

### PR-07 — NetMessage 拡張 + PeerInfo

**ステータス:** `✅ Done`
**依存:** PR-02
**ブランチ:** `feat/net-message-extension`

**変更ファイル:**
```
src/message.rs   PeerInfo, RoomId, MemberId, SkillResultPayload 追加
                 NetMessage に PeerInfo/RoomCreate/RoomJoin/SkillResult variant 追加
src/application.rs  HelloUser 受信時に PeerInfo も処理
```

**やること:**
- [x] `NetMessage` に新 variant を **末尾に追加** (bincode 互換)
  ```
  PeerInfo(PeerInfo)
  RoomCreate(RoomId, Vec<MemberId>)
  RoomJoin(RoomId)
  SkillResult(SkillResultPayload)
  ```
- [x] `PeerInfo`, `SkillResultPayload` 型を定義
- [x] `process_network_message()` に新 variant のマッチアームを追加 (暫定: ログ出力のみ)
- [x] **後方互換テスト:** 旧 `HelloUser` だけを送るノードと接続できること

**リスク:** bincode は enum variant の追加順序に依存
**対策:** 既存 variant の削除・並べ替えは禁止。追加のみ

**受け入れ基準:**
- [x] `cargo build` が通る
- [x] 既存の HelloLan/HelloUser/UserMessage フローが壊れていない
- [x] `NetMessage::PeerInfo` をエンコード→デコードできる (ユニットテスト)

---

### PR-08 — Room エンジン + LAN discovery 動作確認

**ステータス:** `✅ Done`
**依存:** PR-07
**ブランチ:** `feat/room-engine`

**変更ファイル:**
```
src/room/mod.rs      新規 (Room, RoomEngine)
src/room/member.rs   新規 (Member, MemberKind)
src/state.rs         peers, rooms, active_room_id 追加
src/application.rs   PeerInfo 交換フロー, RoomCreate/Join 処理
src/commands/room_cmd.rs  新規 (/room create|list|switch, /peers)
tests/network_integration.rs  新規
```

**やること:**
- [x] `Room`, `Member`, `MemberKind`, `RoomEngine` を実装
- [x] `AppState` に `peers: HashMap<Endpoint, PeerInfo>`, `rooms: Vec<Room>`, `active_room_id` を追加
- [x] `HelloUser` 受信後に `NetMessage::PeerInfo` を自動送信するフローを追加
- [x] `RoomCreate` 送信 → `RoomJoin` 受信 → ルーム確立
- [x] `/room create @user1 [--ai clerk]` コマンドを実装
  - AI participant を自動アタッチ (v1: 起動ノード上で動作)
- [x] `/room list`, `/room switch`, `/peers` コマンドを実装
- [x] `tests/network_integration.rs` — 2プロセス起動して peer discovery が 3秒以内に成立することを確認

**受け入れ基準:**
- [x] 2台で起動し peer discovery が 3秒以内に成立する
- [x] `/room create @user1` でルームが作られ AI が付与される
- [x] `/peers` で発見済みピアが表示される

---

### PR-09 — SkillRegistry (スキャン + frontmatter パース)

**ステータス:** `✅ Done`
**依存:** PR-08
**ブランチ:** `feat/skill-registry`

**変更ファイル:**
```
Cargo.toml              toml (既存) を frontmatter パースに流用確認
src/skill/mod.rs        新規
src/skill/registry.rs   新規
src/state.rs            skill_registry フィールド追加
src/commands/skill_cmd.rs  新規 (/skills)
tests/skill_registry.rs 新規
tests/fixtures/.claude/skills/review-auth/SKILL.md  新規
tests/fixtures/.claude/skills/inspect-amr/SKILL.md  新規
```

**やること:**
- [x] `SkillMeta`, `SkillScope`, `InvokeMode`, `RiskLevel` を定義
- [x] `SkillRegistry::scan(workspace)` を実装:
  - `.claude/skills/` がなければ `Self::empty()` を返す (エラーにしない)
  - 各 `SKILL.md` の YAML frontmatter を `toml::from_str` でパース
  - パースエラーは warn ログ + スキップ
  - `skills_cache.json` に mtime キャッシュ
- [x] `/skills` コマンド — 一覧を表形式で表示 (name / risk / invoke_mode / description)
- [x] テスト用フィクスチャの SKILL.md を作成
- [x] `tests/skill_registry.rs` — スキャン・パース・キャッシュのユニットテスト

**受け入れ基準:**
- [x] `.claude/skills/` がない場合でも起動・動作する
- [x] `/skills` でスキャン結果が表示される
- [x] frontmatter パースエラーの SKILL.md がある場合、他のスキルは正常に読める

---

### PR-10 — SkillExecutor + 承認 UI (非同期実行)

**ステータス:** `✅ Done`
**依存:** PR-09
**ブランチ:** `feat/skill-executor`

**変更ファイル:**
```
src/skill/executor.rs      新規
src/state.rs               pending_confirmation: Option<SkillMeta> 追加
src/application.rs         Signal::SkillDone 処理, y/n キー処理
src/commands/skill_cmd.rs  /skill <name> [args], /run, /cancel 追加
src/ui/chat_panel.rs       承認プロンプト表示
tests/skill_executor.rs    新規
```

**やること:**
- [x] `SkillExecutor::run(meta, args, handle, sidecar)` を実装:
  - skill タイムアウト: 60秒
  - `Signal::SkillDone(SkillResultPayload)` を送信
- [x] `/skill <name> [args]` コマンド:
  - `InvokeMode::Confirm` → `state.pending_confirmation = Some(meta)` → TUI に表示
  - `InvokeMode::AutoSafe` (risk:low のみ) → 直接実行
  - `InvokeMode::Manual` → `/skill` からのみ起動 (Confirm と同じフロー)
  - `InvokeMode::Suggest` → 提案のみ、実行しない
- [x] Enter で `pending_confirmation` がある場合、`y/n` キー処理を追加
- [x] `process_skill_done()` — 結果をチャットに投稿、`state.ai_state = Idle`
- [x] `/run <id>` — AI が提案したスキルを番号で実行
- [x] `/cancel` — `abort_handle.abort()` で実行中タスクを中止
- [x] `tests/skill_executor.rs` — confirm → 実行 → SkillDone の状態遷移テスト

**受け入れ基準:**
- [x] `/skill review-auth` で承認ダイアログが出る
- [x] `y` で非同期実行が開始、実行中は `[acting]` 表示になる
- [x] 実行結果がチャットに投稿される
- [x] `risk:medium` スキルは確認なしに実行されない
- [x] `/cancel` で実行中タスクが止まる

---

### PR-11 — /send ファイル送信移植

**ステータス:** `✅ Done`
**依存:** PR-10
**ブランチ:** `feat/send-file`

**変更ファイル:**
```
src/commands/send_file.rs   termchat から移植 (コマンドプレフィックス対応)
src/commands/mod.rs         SendFileCommand 登録
```

**やること:**
- [x] termchat の `src/commands/send_file.rs` を移植
  - `?send` → `/send` に変更 (COMMAND_PREFIX 変更で自動対応済みのはず)
- [x] ファイル受信時のシステムメッセージをそのまま維持
- [x] `/send` のヘルプテキストを更新

**受け入れ基準:**
- [x] `/send ./error.log` でファイルを送信できる
- [x] 相手側に受信完了メッセージが出る

---

### PR-12 — Transcript JSONL 保存 (Phase 1 完了)

**ステータス:** `✅ Done`
**依存:** PR-10
**ブランチ:** `feat/transcript`

**変更ファイル:**
```
src/room/transcript.rs  新規
src/state.rs            transcript: Option<TranscriptWriter> 追加
src/application.rs      メッセージ追加時に transcript.append() を呼ぶ
tests/transcript.rs     新規
```

**やること:**
- [x] `TranscriptWriter` を実装:
  - 保存先: `~/.local/share/triadchat/transcripts/<room_id>.jsonl`
  - `create_dir_all` でディレクトリを自動作成
  - append mode、`Drop` で flush
- [x] `TranscriptEntry` を定義 (message.rs の型を再利用)
- [x] `Application` でメッセージ追加のたびに `transcript.append()` を呼ぶ
  - write 失敗は warn ログのみ (チャットを止めない)
- [x] `tests/transcript.rs` — JSONL ラウンドトリップテスト

**受け入れ基準 (Phase 1 完了ライン):**
- [x] セッション後に `~/.local/share/triadchat/transcripts/<room_id>.jsonl` が存在する
- [x] JSONL が正しいフォーマットで書かれている
- [x] Transcript write 失敗でチャットが止まらない (テスト)
- [x] Phase 1 受け入れ基準 全項目が通る

---

## Phase 2 — 差別化 (Avatar + 3ペイン)

> 目標: ASCII avatar + プラグインシステム + 3ペイン TUI
> Phase 1 完了 (PR-12 マージ) 後に着手する

---

### PR-13 — AvatarPlugin トレイト + builtin プリセット

**ステータス:** `✅ Done`
**依存:** PR-06 (Phase 0 完了後に着手可能)
**ブランチ:** `feat/avatar-plugin-trait`

**変更ファイル:**
```
src/avatar/mod.rs       新規 (AvatarPlugin trait, AvatarSize, AvatarPluginVTable)
src/avatar/builtin.rs   新規 (human_default, ai_default, robot_guardian)
tests/avatar_builtin.rs 新規
```

**やること:**
- [x] `AvatarPlugin` トレイトを定義
- [x] `AvatarPluginVTable` (`#[repr(C)]` FFI vtable) を定義
  - vtable にバージョン番号フィールドを含める
- [x] `builtin.rs` に組み込みプリセットを実装:
  - `human_default` (3サイズ)
  - `ai_default` (state 別 × 3サイズ)
  - `robot_guardian` (state 別 × 3サイズ)
- [x] `AvatarSize::Compact` を 80列未満で自動選択するロジック
- [x] `tests/avatar_builtin.rs` — 全プリセット × 全 state × 全サイズの組み合わせテスト

**受け入れ基準:**
- [x] 全プリセットが全 state × サイズでパニックせず文字列を返す
- [x] `AvatarPlugin` トレイトを外部クレートで実装できるインターフェースになっている

---

### PR-14 — 3ペイン TUI (peers_panel + status_panel)

**ステータス:** `✅ Done`
**依存:** PR-13
**ブランチ:** `feat/three-pane-tui`

**変更ファイル:**
```
src/ui/mod.rs           3ペインレイアウトに変更
src/ui/peers_panel.rs   新規
src/ui/status_panel.rs  新規
src/ui/chat_panel.rs    既存を分離
tests/ui_layout.rs      新規
```

**やること:**
- [x] `src/ui/mod.rs` を3ペインに変更:
  ```rust
  Layout::horizontal([
      Constraint::Length(18),  // peers
      Constraint::Min(0),      // chat
      Constraint::Length(22),  // status
  ])
  ```
- [x] `peers_panel::draw()` — peer 一覧 + avatar (compact) + presence
  - presence: `online` / `idle` / `busy` / `offline`
- [x] `status_panel::draw()` — AI avatar (normal) + mode + state + TODO 直近5件 + スキル提案
- [x] ターミナル幅 < 80列のとき、peers / status パネルを非表示にしてフォールバック
- [x] `tests/ui_layout.rs` — `ui-test` feature を使ったサイズ計算テスト

**受け入れ基準:**
- [x] 3ペインが表示される
- [x] AI state 変化が peers_panel と status_panel 両方に反映される
- [x] 幅不足時に2ペインにフォールバックする

---

### PR-15 — AvatarManager (libloading による動的ロード)

**ステータス:** `✅ Done`
**依存:** PR-14
**ブランチ:** `feat/avatar-manager`

**変更ファイル:**
```
Cargo.toml              libloading = "0.8" 追加
src/avatar/loader.rs    新規 (AvatarManager)
src/application.rs      AvatarManager 初期化
tests/avatar_plugin.rs  新規 (テスト用 dylib)
```

**やること:**
- [x] `AvatarManager` を実装:
  - `~/.config/triadchat/avatars/*.so|*.dylib` を走査
  - `libloading::Library` でロード
  - vtable バージョン確認 (不一致は warn + スキップ)
  - `render()` — 外部プラグイン優先、なければ builtin
  - `list_all_presets()` — 外部 + builtin をマージ、重複除去
- [x] `tests/avatar_plugin.rs` — テスト用の最小 dylib を作成し、ロード + render が動くことを確認

**リスク:** FFI ABI の安定性
**対策:** `#[repr(C)]` + `extern "C"` + vtable バージョンフィールドで管理

**受け入れ基準:**
- [x] `~/.config/triadchat/avatars/` に dylib を置くとロードされる
- [x] vtable バージョン不一致の dylib はスキップされる
- [x] builtin プリセットは常にフォールバックとして機能する

---

### PR-16 — /avatar コマンド (Phase 2 完了)

**ステータス:** `✅ Done`
**依存:** PR-15
**ブランチ:** `feat/avatar-commands`

**変更ファイル:**
```
src/commands/avatar_cmd.rs  新規
src/state.rs                user_avatar, ai_avatar フィールド追加
src/config.rs               user.avatar 設定の反映
```

**やること:**
- [x] `/avatar set <target> <preset>` — 対象の avatar preset を変更
- [x] `/avatar preview` — 現在のアバターを全サイズで表示
- [x] `/avatar mode <compact|normal|expressive>` — グローバルサイズ変更
- [x] `/avatar list` — `AvatarManager::list_all_presets()` の結果を表示
- [x] `config.toml` の `user.avatar` を起動時に反映

**受け入れ基準 (Phase 2 完了ライン):**
- [x] `/avatar set @ops-ai robot_guardian` でアバターが変わる
- [x] `/avatar list` にプラグインのプリセットが出る
- [x] ASCII avatar が AI state に応じて変化する
- [x] Phase 2 受け入れ基準 全項目が通る

---

## サマリテーブル

| PR | タイトル | フェーズ | 依存 | ステータス |
|----|----------|----------|------|-----------|
| PR-01 | fork + rename + prefix 変更 | 0 | — | `✅ Done` |
| PR-02 | tokio Runtime + Signal 拡張 | 0 | PR-01 | `✅ Done` |
| PR-03 | SidecarAdapter | 0 | PR-02 | `✅ Done` |
| PR-04a | 言語設定 (LanguageConfig + UI i18n) | 0 | PR-03 | `✅ Done` |
| PR-04 | prompt.rs + パーサー ★ | 0 | PR-04a | `✅ Done` |
| PR-05 | AiMediator + 介入トリガー | 0 | PR-04 | `✅ Done` |
| PR-06 | /summary /todos /decisions | 0 | PR-05 | `✅ Done` |
| PR-07 | NetMessage 拡張 + PeerInfo | 1 | PR-02 | `✅ Done` |
| PR-08 | Room エンジン + LAN discovery | 1 | PR-07 | `✅ Done` |
| PR-09 | SkillRegistry | 1 | PR-08 | `✅ Done` |
| PR-10 | SkillExecutor + 承認 UI | 1 | PR-09 | `✅ Done` |
| PR-11 | /send 移植 | 1 | PR-10 | `✅ Done` |
| PR-12 | Transcript JSONL | 1 | PR-10 | `✅ Done` |
| PR-13 | AvatarPlugin trait + builtin | 2 | PR-06 | `✅ Done` |
| PR-14 | 3ペイン TUI | 2 | PR-13 | `✅ Done` |
| PR-15 | AvatarManager (libloading) | 2 | PR-14 | `✅ Done` |
| PR-16 | /avatar コマンド | 2 | PR-15 | `✅ Done` |

**並列実行可能な組み合わせ:**
- PR-07 は PR-02 完了後に PR-03 と並列着手可能
- PR-11 と PR-12 は PR-10 完了後に並列着手可能
- PR-13 は PR-06 完了後に PR-07 と並列着手可能
