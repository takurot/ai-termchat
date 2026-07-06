# ai-termchat (triadchat) 機能仕様書

版: v0.3  
ベース: termchat v1.3.1 fork  
言語: Rust (edition 2021)

---

## 1. 概要

termchat を fork し、「人間 + AI」または「人間A・人間B・AI」の会話型オペレーション端末へ拡張する。

**コアバリュー:** ターミナルから出ずに会話するだけで TODO・決定事項が構造化され、Claude Code skills に接続できる。

**primary use case は 1人+AI。** 2人+AI はその自然な拡張として Phase 1 で実現する。  
avatar・プラグインは差別化要素であり、clerk mode の精度検証が先決。

---

## 2. 開発フェーズ

| フェーズ | スコープ | 成功基準 |
|----------|----------|----------|
| **Phase 0** (PoC) | シングルノード。1人+AI の2者チャット。`/summary` `/todos` `/decisions` | 自分で打ち込んで TODO が正確に出ること |
| **Phase 1** (MVP) | LAN 2台。2人+AI の3者 Room。`/skill` 実行 + 承認 UI | 2台で動く + 1 skill が非同期で実行できること |
| **Phase 2** | ASCII avatar + AvatarPlugin トレイト + status panel | `/avatar list` にプラグインが出ること |

**Phase 0 の唯一の成功条件:** `/summary` が使えること。プロンプト設計の精度が全体クオリティを決定する。他は全部後回し。

---

## 3. termchat との差分サマリ

| 項目 | termchat | ai-termchat |
|------|----------|-------------|
| コマンドプレフィックス | `?` | `/` |
| ルームモデル | 全員フラット | 2者 (Phase 0) → 3者 Room (Phase 1) |
| メッセージ種別 | HelloLan/HelloUser/UserMessage/UserData/Stream | 上記に加え PeerInfo/RoomCreate/RoomJoin/AiMessage/SkillResult |
| UI レイアウト | 2ペイン (chat + input) | Phase 0: 2ペイン維持 → Phase 2: 3ペイン |
| アバター | なし | Phase 0-1: 名前の色分けのみ → Phase 2: ASCII avatar plugin |
| AI | なし | AiMediator (非同期) + Claude Code sidecar |
| AI 応答表示 | — | 実行中は `[ops-ai: thinking...]` をチャットに表示 |
| 設定ファイルパス | `~/.config/termchat/config` | `~/.config/triadchat/config.toml` |
| Cargo package name | termchat | triadchat |
| video stream | Linux optional feature | 除外 |

---

## 4. クレート構成

Phase 0 で作るもの、Phase 1/2 で追加するものを明示する。

```
triadchat/
├── Cargo.toml
└── src/
    ├── main.rs                  # CLI エントリポイント
    ├── lib.rs                   # pub re-exports
    ├── config.rs                # Config + Theme (TOML)
    ├── message.rs               # NetMessage (serde + bincode)
    ├── state.rs                 # State (旧称 AppState)
    ├── action.rs                # Action trait (termchat 継承)
    ├── encoder.rs               # bincode encode/decode (termchat 継承)
    ├── terminal_events.rs       # crossterm イベント収集 (termchat 継承)
    ├── renderer.rs              # Renderer (termchat 継承)
    ├── application.rs           # Application メインループ
    │
    ├── ai/                      # [Phase 0]
    │   ├── mod.rs               # AiMediator
    │   ├── prompt.rs            # プロンプトテンプレート (精度の核心)
    │   ├── sidecar.rs           # Claude Code sidecar adapter (claude -p)
    │   ├── classifier.rs        # 発言分類 (discuss/decide/task/execute)
    │   └── trigger.rs           # 自動介入トリガー判定
    │
    ├── room/                    # [Phase 1]
    │   ├── mod.rs               # Room, RoomEngine
    │   ├── member.rs            # Member (human | ai)
    │   └── transcript.rs        # Transcript (JSONL 永続化)
    │
    ├── skill/                   # [Phase 1]
    │   ├── mod.rs               # SkillBridge
    │   ├── registry.rs          # .claude/skills/ スキャン
    │   └── executor.rs          # 非同期実行 + Signal::SkillDone
    │
    ├── avatar/                  # [Phase 2]
    │   ├── mod.rs               # AvatarPlugin trait + AvatarManager
    │   ├── builtin.rs           # 組み込みプリセット
    │   └── loader.rs            # libloading による動的ロード
    │
    ├── commands/
    │   ├── mod.rs               # CommandManager (prefix `/`)
    │   ├── send_file.rs         # /send (termchat 移植) [Phase 1]
    │   ├── ai_cmd.rs            # /ai mode|quiet [Phase 0]
    │   ├── summary_cmd.rs       # /summary /todos /decisions [Phase 0]
    │   ├── room_cmd.rs          # /room create|switch|list [Phase 1]
    │   ├── skill_cmd.rs         # /skills /skill <name> [Phase 1]
    │   └── avatar_cmd.rs        # /avatar set|preview|mode|list [Phase 2]
    │
    └── ui/
        ├── mod.rs               # draw() エントリ
        ├── chat_panel.rs        # 会話本体 (termchat 移植、Phase 0 から)
        ├── peers_panel.rs       # 左: peer 一覧 [Phase 2]
        └── status_panel.rs      # 右: AI ステータス + skill 候補 [Phase 2]
```

---

## 5. Cargo.toml 依存関係

依存関係の真実のソースは `Cargo.toml` を参照すること（バージョンは都度更新されるため、ここに転記しない）。

本節は設計上の注記のみを記載する。

### 主要依存と役割

| 依存 | 役割 | 備考 |
|------|------|------|
| `message-io` | ネットワーク (UDP mcast + TCP) | termchat 継承 |
| `bincode` | wire シリアライズ | **後述の破壊的変更参照** |
| `serde` / `serde_json` / `serde_yaml` | 設定・AI 出力のシリアライズ | |
| `ratatui` + `crossterm` | TUI レンダリング + 端末イベント | **ratatui 0.26 に移行済み**（termchat 由来の `tui 0.14` から） |
| `tokio` | AI sidecar の非同期実行 | `Runtime::new()` + `Handle::spawn()` で message-io スレッドと分離 |
| `ed25519-dalek` / `x25519-dalek` / `chacha20poly1305` | peer 認証署名 + トランスポート暗号化 | Phase 1 セキュリティ |
| `clap` | CLI 引数 | |
| `libloading` | AvatarPlugin 動的ロード (optional: `avatar-ffi` feature) | |

### bincode 1 → 2 wire 互換性（重要）

`bincode` を `1.3` から `2.0.0-rc.3` に移行した。`bincode::config::legacy()` を用いて termchat 互換のワイヤーフォーマットを維持しているが、新しいエントリポイント (`bincode::serde::encode_to_vec` / `decode_from_slice`) を使用する。enum variant は **末尾追加のみ**（削除・並べ替え禁止）を維持することで後方互換を保つ。

> **MSRV:** Rust 1.82（`Cargo.toml` の `rust-version` で宣言。`Option::is_none_or` 等 1.82 安定化 API を使用するため 1.75 から引き上げ）。CI での強制は別途 GitHub Issue で対応。

### バージョン

`Cargo.toml` の `version` が真実のソース。現在 `0.1.x` 系列。

---

## 6. ネットワークプロトコル

### 6.1 NetMessage 拡張

以下は `src/message.rs` の `NetMessage` から転記した実装の真実。variant は追加順（= bincode のディスク順序）に並べること。削除・並べ替えは後方互換性を壊すため禁止。

```rust
/// src/message.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NetMessage {
    // --- termchat 継承 (Phase 0) ---
    HelloLan(String, u16),            // user_name, server_port
    HelloUser(String),                // user_name
    UserMessage(String),              // content
    UserData(String, Chunk),          // file_name, chunk
    AiMessage(AiPayload),             // Phase 0 追加: AI が生成したメッセージ

    // --- Phase 1: ルーム / peer / skill ---
    PeerInfo(PeerInfo),               // ノードのメタ情報交換
    RoomCreate(RoomId, Vec<MemberId>),// ルーム作成・参加招待
    RoomCreateV2 {                    // ai_mode 付きルーム作成
        room_id: RoomId,
        members: Vec<MemberId>,
        ai_mode: Option<AiMode>,
    },
    RoomJoin(RoomId),
    SkillResult(SkillResultPayload),  // skill 実行結果

    // --- Phase 1: トランスポートセキュリティ ---
    PeerIdentity {                    // ed25519 身元証明 (署名検証の対象)
        public_key: Vec<u8>,
        signature: Vec<u8>,
        timestamp: u64,
    },
    KeyExchange {                     // x25519 公開鍵 + ed25519 署名
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
    Secure(Vec<u8>),                  // ChaCha20Poly1305 で暗号化された内側 NetMessage

    // --- ファイル転送 (オファー/承認/却下/キャンセル) ---
    TransferOffer { file_name: String, file_size: u64, sender: String },
    TransferAccept { file_name: String },
    TransferReject { file_name: String, reason: String },
    TransferCancel { file_name: String },
}
```

補助型（いずれも `src/message.rs`）:

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPayload {
    pub text: String,
    pub intent: AiIntent,
    pub structured: Option<StructuredOutput>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiIntent {
    #[default]
    Clarify,
    Summary,
    Todo,
    Decision,
    SkillSuggest,
    Skip,                 // 介入不要時のパーサーフォールバック
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredOutput {
    pub todos: Vec<TodoItem>,
    pub decisions: Vec<String>,
    pub skill_suggestions: Vec<String>,
    pub raw_text: Option<String>,     // パース失敗時の raw fallback (执行候補抽出からは除外)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TodoItem {
    pub text: String,
    pub assignee: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub user_name: String,
    pub server_port: u16,
    pub node_version: String,
    #[serde(default)]
    pub avatar: String,               // Phase 1: avatar preset 名
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillResultPayload {
    pub skill_name: String,
    pub summary: String,
    pub success: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Chunk {
    Data(Vec<u8>),
    Error,
    End,
}

pub type RoomId = String;
pub type MemberId = String;
```

> すべての peer 由来 payload には `MAX_*` 長さ上限と `validate()` が定義されている（`src/message.rs` を参照）。`NetMessage::Secure` のみ署名検証済みかつ鍵交換済み endpoint から受理する。

### 6.2 ピア探索シーケンス (Phase 1)

termchat と同一。Phase 0 ではシングルノードのためスキップ。

```
Node A                         Node B
  |--- HelloLan (UDP mcast) -->  |
  |<-- HelloLan (UDP mcast) ---  |
  |--- TCP connect ----------->  |
  |--- PeerInfo --------------->  |
  |<-- PeerInfo ----------------  |
  |         (ピア確立)             |
```

---

## 7. State

アプリケーションの中央可変状態は `pub struct State`（`src/state.rs`）。全フィールドの正確な定義は同ファイルを真実のソースとすること。主要なグループのみ以下に示す。

```rust
/// src/state.rs (抜粋 — 全フィールドは実装を参照)
pub struct State {
    // --- termchat 継承 ---
    messages: Vec<ChatMessage>,
    scroll_messages_view: usize,
    input: Vec<char>,
    input_cursor: usize,
    input_history: Vec<String>,        // Up/Down で遡る送信履歴
    local_user_name: String,
    lan_users / peers / users_id,      // peer 管理
    rooms / active_room_id,            // Phase 1: ルーム状態

    // --- Phase 0 AI ---
    ai_state: AiState,
    ai_mode: AiMode,
    ai_thinking: bool,
    abort_handle: Option<tokio::task::AbortHandle>,  // spawn リーク防止

    // --- Phase 1 skill / security ---
    pending_confirmation / skill_proposals,
    transcript: Option<TranscriptWriter>,
    // active_transfers / pending_transfer_offers (ファイル転送)
}
```

> **命名:** 設計段階の `AppState` は実装では `State` に改名されている。ドキュメント・コードともに `State` を使用すること。

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiState {
    Idle,
    Thinking,   // [ops-ai: thinking...] 表示
    Acting,     // skill 実行中
    Disabled,   // AI 利用不可 (claude コマンド不在等)
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiMode {
    Clerk,
    Listener,
    Moderator,
    Operator,
    Companion,  // 全メッセージに会話的に応答
}
```

> `AiState` は `Copy` ではない（`Failed(String)` を持つため）。UI の描画は `&AiState` の参照で行う。

---

## 8. Application メインループ

termchat の `Signal` に AI 関連を追加する。AI 呼び出しは tokio タスクで非同期実行し、結果を signal で戻す。

```rust
/// src/application.rs
pub enum Signal {
    Terminal(TermEvent),
    Action(Box<dyn Action>),
    AiResponse(AiPayload),               // Phase 0: AI 応答が返ってきた
    SkillDone(SkillResultPayload),        // Phase 1: skill 実行完了
    Close(Option<anyhow::Error>),
}
```

**非同期 AI 呼び出しフロー:**

```
ユーザーがメッセージ送信
  → trigger::should_intervene() で介入判定
  → true → state.ai_thinking = true → TUI に "[ops-ai: thinking...]" 表示
  → tokio::spawn(sidecar.ask(context, prompt))
  → 完了 → node.signals().send(Signal::AiResponse(payload))
   → process_ai_response() → state.ai_thinking = false → メッセージ追加 → TUI     更新
```

`ai_thinking = true` の間は同じメッセージに対して再トリガーしない (二重呼び出し防止)。

**tokio と message-io の統合方針:**  
message-io は独自スレッドを持つため `#[tokio::main]` との競合を避ける。  
`Runtime::new()` で tokio ランタイムを手動生成し、`Handle::spawn()` で AI タスクを投げる。

```rust
// Application::new() 内
let rt = tokio::runtime::Runtime::new()?;
let handle = rt.handle().clone();
// spawn 時
handle.spawn(async move {
    let payload = sidecar.ask(&prompt).await;
    node_handler.signals().send(Signal::AiResponse(payload));
});
```

---

## 8a. コンポーネント依存グラフ

```
┌────────────────────────────────────────────────────────────┐
│                      Application                           │
│  EventLoop: message-io NodeHandler<Signal>                 │
│                                                            │
│  Signal:                                                   │
│    Terminal(TermEvent)       ← crossterm                   │
│    Action(Box<dyn Action>)   ← termchat 継承               │
│    AiResponse(AiPayload)     ← tokio::spawn 結果           │
│    SkillDone(SkillResult)    ← tokio::spawn 結果 [P1]      │
│    Close(Option<Error>)                                    │
└──────┬──────────────┬─────────────────┬───────────────────┘
       │              │                 │
  ┌────▼────┐   ┌─────▼──────┐   ┌─────▼──────────┐
  │State    │   │AiMediator  │   │SkillBridge [P1]│
  │         │   │            │   │SkillRegistry   │
  │messages │   │trigger     │   │SidecarAdapter  │
  │ai_state │   │classifier  │   │(共有)          │
  │ai_think │   │prompt.rs ◄─┼───┤                │
  │rooms[P1]│   │            │   └────────────────┘
  │pending  │   └─────┬──────┘
  │confirm  │         │ tokio::spawn (Handle)
  └────┬────┘   ┌─────▼──────────────────┐
       │        │SidecarAdapter           │
  ┌────▼────┐   │claude -p (subprocess)  │
  │  UI     │   │timeout: 30s            │
  │draw()   │   └────────────────────────┘
  │chat     │
  │peers[P2]│
  │status[P2│
  └─────────┘

インターフェース境界:
  Terminal → App       crossterm::Event (in)
  App → TUI            &State read-only borrow (out)
  App → AI             tokio Handle::spawn + signals() (async)
  App → Skill [P1]     tokio Handle::spawn + signals() (async)
  App ↔ Network [P1]   message-io NetMessage/bincode (bidirectional)
  AI → Storage [P1]    TranscriptWriter append JSONL (out)
```

---

## 8b. 失敗モードと回復戦略

### AI 呼び出し

| 失敗 | 検出 | 回復 |
|------|------|------|
| `claude` コマンドが存在しない | 起動時 `which::which("claude")` | エラーメッセージ表示 + `ai.enabled = false` ⇒ `AiState::Disabled`（`AiProvider::Disabled` という variant は存在しない）|
| sidecar タイムアウト (30s) | `tokio::time::timeout` | `AiState::Failed` 表示、次メッセージで再トリガー可能 |
| stdout が空 | `trim().is_empty()` チェック | `AiState::Failed` + "no response" メッセージ |
| spawn 中に `ai_thinking` リーク | `JoinHandle` の `Drop` で `ai_thinking = false` | `AbortHandle` を `State` に持たせてガード |
| プロンプト生成時に transcript が空 | `if transcript.is_empty() { return false }` | 介入しない (エラーにしない) |

### Skill 実行 (Phase 1)

| 失敗 | 検出 | 回復 |
|------|------|------|
| SKILL.md が存在しない | fs scan 時に除外 | `/skills` に出ない |
| frontmatter パースエラー | `toml::from_str` Err | warn ログに落としてスキップ |
| skill タイムアウト | 60秒 (skill は長め) | `AiState::Failed` + エラーメッセージ |
| `trusted_peers` 外からの実行 | peer fingerprint チェック | "permission denied" メッセージ |

### ネットワーク (Phase 1)

| 失敗 | 検出 | 回復 |
|------|------|------|
| bincode デシリアライズ失敗 | `encoder::decode` が `None` | 当該メッセージを無視、warn ログ |
| 旧バージョンの `HelloUser` 受信 | variant マッチ | `PeerInfo` なしで peer 登録 (後方互換) |
| Transcript write 失敗 | `writeln!` Err | warn ログのみ (チャットを止めない) |

---

## 8c. ストレージ・バリデーション境界

### SidecarAdapter — 唯一の外部プロセス I/O

```rust
impl SidecarAdapter {
    /// 起動時に claude コマンドの存在確認
    pub fn new(workspace: PathBuf) -> anyhow::Result<Self> {
        which::which("claude")
            .context("claude CLI not found. Install: npm i -g @anthropic-ai/claude-code")?;
        anyhow::ensure!(workspace.exists(), "workspace not found: {}", workspace.display());
        Ok(Self { workspace })
    }
}

// 入力: prompt 最大 50,000 文字 (超過時は先頭から切り詰め)
// 出力: trim 済み非空文字列、空は Err 扱い
```

### SkillRegistry — ファイルシステム境界

```rust
impl SkillRegistry {
    pub fn scan(workspace: &Path) -> Self {
        let dir = workspace.join(".claude/skills");
        // ディレクトリが存在しなければ空の registry を返す (エラーにしない)
        if !dir.exists() { return Self::empty(); }
        // frontmatter パースエラーは warn ログに落として continue
        // 結果を skills_cache.json に mtime でキャッシュ
    }
}
```

### TranscriptWriter — ストレージ境界 (Phase 1)

```rust
pub struct TranscriptWriter {
    file: std::fs::File,  // append mode、Drop 時に flush
}

impl TranscriptWriter {
    pub fn open(room_id: &str) -> anyhow::Result<Self> {
        let dir = dirs_next::data_dir()
            .context("no data dir")?
            .join("triadchat/transcripts");
        std::fs::create_dir_all(&dir)?;
        let file = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open(dir.join(format!("{}.jsonl", room_id)))?;
        Ok(Self { file })
    }
    // write 失敗は呼び出し元が warn ログのみ (チャットを止めない)
}
```

### Config — バリデーション境界

```rust
impl Config {
    pub fn load() -> Self {
        Self::from_file().unwrap_or_else(|e| {
            eprintln!("config load failed ({}), using defaults", e);
            Self::default()
        })
    }
    fn validate(&self) -> Vec<String> {
        let mut errors = vec![];
        if self.ai.cooldown_secs == 0 {
            errors.push("ai.cooldown_secs must be > 0".into());
        }
        errors  // 起動時に stderr 出力、エラーがあってもデフォルト値で継続
    }
}
```

---

## 9. AI Mediator

### 9.1 プロンプト設計 (Phase 0 の最優先事項)

**`/summary` `/todos` `/decisions` の精度が全体の価値を決定する。** プロンプトは `src/ai/prompt.rs` に集約し、反復的に改善する。
この節のコードブロックは初期設計時の疑似コードであり、現在のセキュリティ上の実装契約は 9.3a を優先する。

すべてのプロンプト関数は `lang: &str` (例: `"ja"`, `"en"`) を受け取り、  
AI への出力言語指示を末尾に付加する。これにより `config.toml` の `[language]` 設定が AI 応答言語を制御する。

```rust
/// src/ai/prompt.rs

/// 言語コードを自然言語名に変換
fn lang_instruction(lang: &str) -> &'static str {
    match lang {
        "ja" => "必ず日本語で出力してください。",
        "en" => "Respond in English.",
        "zh" => "请用中文回答。",
        "ko" => "한국어로 답변해 주세요.",
        _    => "Respond in English.",  // デフォルト
    }
}

pub fn summary_prompt(transcript: &str, lang: &str) -> String {
    format!(r#"
以下はエンジニアの会話ログです。
会話を読んで、以下を簡潔に出力してください。
{lang_instruction}

## 要約
(3文以内)

## 決定事項
- (箇条書き。決まったことだけ。未決は含めない)

## TODO
- [担当者名]: タスク内容  ← 担当者が会話から読み取れる場合のみ付ける

会話:
{transcript}

出力は上記フォーマットのみ。説明文は不要。
"#, transcript = transcript)
}

pub fn intervene_prompt(transcript: &str, last_messages: &[&str]) -> String {
    format!(r#"
あなたはチャットの clerk です。
以下の会話の流れを読み、介入が必要か判断してください。

介入条件:
- 決定・TODO・担当・期限が出てきたとき
- 重要な曖昧さや矛盾があるとき
- 明示的に呼ばれたとき (/ai, /summary 等)

介入不要:
- 雑談・あいさつ
- 直前30秒以内に発言済み
- 会話のテンポを壊す場合

直近の発言:
{last_messages}

全会話:
{transcript}

介入が必要なら以下フォーマットで出力。不要なら "SKIP" のみ出力。

[出力フォーマット]
INTENT: summary|todo|decision|clarify|skill_suggest
TEXT: (自然文、2-3文)
STRUCTURED:
  todos: ["担当者: タスク", ...]
  decisions: ["決定事項", ...]
  skills: ["skill名", ...]
"#,
        last_messages = last_messages.join("\n"),
        transcript = transcript,
    )
}
```

### 9.2 介入トリガー

```rust
/// src/ai/trigger.rs
pub struct TriggerConfig {
    /// 直前 N 秒以内に AI が発言済みなら介入しない
    pub cooldown_secs: u64,       // default: 30
    /// 人間が N 連続発言中は介入しない
    pub human_streak_limit: usize, // default: 3
}

pub fn should_intervene(
    msg: &str,
    mode: AiMode,
    ai_thinking: bool,
    last_ai_at: Option<Instant>,
    human_streak: usize,
    cfg: &TriggerConfig,
) -> bool {
    if ai_thinking { return false; }
    if let Some(t) = last_ai_at {
        if t.elapsed().as_secs() < cfg.cooldown_secs { return false; }
    }
    if human_streak >= cfg.human_streak_limit { return false; }

    match mode {
        AiMode::Listener  => false,
        AiMode::Clerk     => contains_decision_marker(msg) || contains_todo_marker(msg),
        AiMode::Moderator => contains_ambiguity(msg) || contains_contradiction(msg),
        AiMode::Operator  => contains_execute_request(msg),
    }
}
```

### 9.3 発言分類

```rust
/// src/ai/classifier.rs
pub enum MessageClass {
    Discuss,
    Decide,
    Task,
    Execute,  // Execute のときだけスキル提案を生成
}
```

### 9.3a Prompt / Structured Output Security

AI prompt に埋め込む会話ログ・直近発言・直接メンション本文は、必ずユーザー由来データとして扱う。

- `transcript`, `question`, `last_messages` は固定タグで囲み、`&`, `<`, `>` をエスケープする。
- ユーザー由来データの行頭に `TASK:`, `INTENT:`, `TEXT:`, `STRUCTURED:`, `TRANSCRIPT:`, `LAST_MESSAGES:`, `QUESTION:` が現れた場合は、LLM 出力制御行に見えないよう neutralize する。
- AI から返る `STRUCTURED` JSON は `StructuredOutput::validate()` を通す。`raw_text` を含む structured payload、空または制御行風の skill 名、改行や空白を含む skill 名は skill proposal として保存しない。
- 同じ validation は parser だけでなく `AiResponse` 処理と `NetMessage::AiMessage` decode にも適用し、parser を迂回した payload でも実行候補を作らない。

### 9.4 Claude Code Sidecar

この節のコードブロックは初期設計時の疑似コードであり、現在の timeout cleanup 契約はコードブロック後の記述を優先する。

```rust
/// src/ai/sidecar.rs
pub struct SidecarAdapter {
    pub workspace: PathBuf,
}

impl SidecarAdapter {
    /// 1-shot 呼び出し (Phase 0, Level 1)
    /// タイムアウト: 30 秒
    pub async fn ask(&self, prompt: &str) -> anyhow::Result<String> {
        let output = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::process::Command::new("claude")
                .arg("-p")
                .arg(&prompt)
                .current_dir(&self.workspace)
                .output(),
        )
        .await
        .context("sidecar timeout")??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("claude -p failed: {}", stderr);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    /// skill 実行 (Phase 1)
    pub async fn run_skill(&self, skill_name: &str, args: &[&str]) -> anyhow::Result<String> {
        let prompt = format!("/{} {}", skill_name, args.join(" "));
        self.ask(&prompt).await
    }
}
```

Sidecar process は timeout 時にバックグラウンドで残してはならない。Unix では sidecar を独立 process group で起動し、timeout 時は process group ごと best-effort で終了させる。

---

## 10. Skill Bridge (Phase 1)

### 10.1 SkillRegistry

`.claude/skills/` を走査し `SKILL.md` の YAML frontmatter を読む。

```rust
/// src/skill/registry.rs
pub struct SkillMeta {
    pub name: String,
    pub scope: SkillScope,
    pub invoke_mode: InvokeMode,
    pub allowed_tools: Vec<String>,
    pub risk: RiskLevel,
    pub description: String,
}

pub enum InvokeMode {
    Manual,
    Confirm,    // 承認ダイアログを出してから実行
    AutoSafe,   // risk:low のみ自動実行
    Suggest,    // 提案のみ、実行しない
}

pub enum RiskLevel { Low, Medium, High }
```

frontmatter 例:
```yaml
---
name: review-auth
invoke: confirm
risk: medium
allowed-tools: [Read, Grep]
description: 認証ロジックをレビューする
---
```

### 10.2 非同期実行フロー

skill 実行は必ず非同期。UI を止めない。

```
/skill review-auth
  → SkillRegistry::find("review-auth") → SkillMeta
  → invoke_mode == Confirm
      → TUI に承認プロンプト "[review-auth] 実行しますか? [y/n]" 表示
      → y → state.ai_state = AiState::Acting → "[ops-ai: /review-auth 実行中...]" 表示
      → tokio::spawn(SidecarAdapter::run_skill("review-auth", []))
      → 完了 → Signal::SkillDone → 結果を room に投稿 → state.ai_state = Idle
      → n → キャンセル
  → invoke_mode == AutoSafe (risk:low)
      → 承認なしで即実行
```

---

## 11. アバタープラグイン (Phase 2)

### 11.1 設計方針

Phase 0-1 では名前の色分けのみで peer 識別する。Phase 2 で ASCII avatar と plugin システムを追加する。

Phase 2 まで `/avatar` コマンドは実装しない。

### 11.2 AvatarPlugin トレイト

```rust
/// src/avatar/mod.rs

pub trait AvatarPlugin: Send + Sync {
    fn list_presets(&self) -> Vec<String>;
    fn render(&self, preset: &str, state: AiState, size: AvatarSize) -> String;
}

#[derive(Clone, Copy)]
pub enum AvatarSize {
    Compact,    // 1行  例: [@@ ]
    Normal,     // 3行
    Expressive, // 5行
}
```

FFI 安定プラグイン向け vtable (`.so` / `.dylib` 対応):

```rust
#[repr(C)]
pub struct AvatarPluginVTable {
    pub list_presets: unsafe extern "C" fn() -> *const *const std::os::raw::c_char,
    pub render: unsafe extern "C" fn(
        preset: *const std::os::raw::c_char,
        state: u8,   // 0=idle 1=listening 2=thinking 3=warning 4=acting 5=failed
        size: u8,    // 0=compact 1=normal 2=expressive
    ) -> *const std::os::raw::c_char,
}
```

### 11.3 組み込みプリセット

```
human_default:
  compact:     (^_^)
  normal:      /-\
               (^_^)
               / \

ai_default:
  idle:        [@@ ]
  thinking:    [.. ]
  warning:     [!! ]
  acting:      [>> ]
  failed:      [xx ]

robot_guardian:
  compact:    [|..|]
  normal:      .----.
              [| .. |]
               | -- |
  expressive:  .-------.
              [|  ..  |]
               |  --  |
              /|______|\
```

### 11.4 AvatarManager

```rust
/// src/avatar/loader.rs
pub struct AvatarManager {
    plugins: HashMap<String, Box<dyn AvatarPlugin>>,
}

impl AvatarManager {
    /// ~/.config/triadchat/avatars/*.so|dylib を走査してロード
    pub fn load(avatar_dir: &Path) -> Self { ... }

    /// 探索順: 外部プラグイン → builtin
    pub fn render(&self, preset: &str, state: AiState, size: AvatarSize) -> String {
        self.plugins
            .get(preset)
            .map(|p| p.render(preset, state, size))
            .unwrap_or_else(|| builtin::render(preset, state, size))
    }

    pub fn list_all_presets(&self) -> Vec<String> {
        let mut presets: Vec<_> = self.plugins.values()
            .flat_map(|p| p.list_presets())
            .collect();
        presets.extend(builtin::PRESETS.iter().map(|s| s.to_string()));
        presets.sort();
        presets.dedup();
        presets
    }
}
```

---

## 12. TUI レイアウト

### Phase 0-1: 2ペイン (termchat 踏襲)

```
┌─────────────────────────────────────────────────────────────┐
│  triadchat — takuro                    [ops-ai: thinking...] │
├─────────────────────────────────────────────────────────────┤
│ 10:01 takuro (me): この関数、責務が多すぎる                  │
│ 10:02 tanaka: 認証周りを切り出したい                        │
│ 10:03 ops-ai ✦ 決定: auth を service 層へ分離               │
│               TODO: takuro — auth 抽出設計                   │
│                     tanaka — 回帰確認                        │
│               提案: [1] /skill review-auth                   │
├─────────────────────────────────────────────────────────────┤
│ > /skill review-auth                                         │
└─────────────────────────────────────────────────────────────┘
```

AI が thinking 中はヘッダーに `[ops-ai: thinking...]` を表示。  
AI の発言行は `✦` マークで他と区別する。

### Phase 2: 3ペイン

```
┌──────────────────────────────────────────────────────────────┐
│  peers                   room: takuro+tanaka+ops-ai          │
├──────────────┬──────────────────────────────┬────────────────┤
│ (^_^) takuro │ 10:01 takuro: この関数重い   │ [.. ] ops-ai   │
│ (-_-) tanaka │ 10:02 tanaka: 分離しよう     │ mode: clerk    │
│ [>> ] ops-ai │ 10:03 ops-ai ✦              │ ─────────────  │
│              │  決定: auth → service 層     │ TODO:          │
│              │  TODO: takuro/tanaka         │ ・auth 設計    │
│              │  提案: [1] /skill review-auth│ ・回帰確認     │
├──────────────┴──────────────────────────────┴────────────────┤
│ > /skill review-auth                                          │
└──────────────────────────────────────────────────────────────┘
```

レイアウト:
```rust
Layout::horizontal([
    Constraint::Length(18),  // peers (Phase 2)
    Constraint::Min(0),      // chat
    Constraint::Length(22),  // AI status (Phase 2)
])

```

ファイル受信中の進捗は ops-ai パネルの右カラムに表示される
（`ActiveTransferView` が `active_transfers_view()` 経由で描画）。
プロトコル変更なしで、受信バイト数のみを常に表示する:

```
Receiving
↓ 1.4 MB  alice: notes.txt
↓ 512 KB  bob: photo.png  … +1 more
```

表示は最大2行 + overflow表示。転送が完了 (`Chunk::End`) またはエラー (`Chunk::Error`) になると消える。
バイトカウンタは `disconnected_user` によるピア切断時にもクリアされる。

デルタ符号: `format_bytes(u64)` — バイナリ単位 (1024)、整数部が10未満の単位は小数1桁、それ以外は整数表示。

---

## 13. コマンド仕様
`CommandManager::COMMAND_PREFIX` を `"?"` → `"/"` に変更。

> **正真のリスト:** アプリ内 `/help`（`src/application/mod.rs::help_text()`）がコマンド群の正。以下はフェーズ別の参照用。グループ分けは `/help` に準ずる。

### AI

```
/ai mode <clerk|listener|moderator|operator|companion>
/ai quiet <on|off>
/ai freq <low|normal|high>
/ai provider <claude|codex|gemini|custom>
```

`/ai provider` は実行中の AI エンジンを動的に切り替える。`SidecarAdapter` が新しいプロバイダーのコマンド (`which` または `ai.command`) を解決できた時点で `state.ai_provider` と `ai_mediator` を置き換える。解決に失敗した場合は以前のプロバイダーと mediator を維持し、エラーメッセージを表示する。

### Summary (Phase 0)

```
/summary          直近会話の要約
/todos            TODO 一覧
/decisions        決定事項一覧
/context          会話コンテキスト全体
```

### Rooms (Phase 1)

```
/room create @user1 [--ai <mode>]   # RoomCreateV2 で AI モードを同梱
/room list
/room switch <id|name>
```

### Peers (Phase 1)

```
/peers
/peer connect <host:port>           # 直接 peer 接続
/trust list                         # 信頼済み fingerprint 一覧
/trust add <peer|fp>                # peer を明示的に信頼
/trust remove <peer|fp>             # 信頼を取り消し
```

### Skills (Phase 1)

```
/skills
/skill <name> [args]
/run <proposal_id>                  # AI が提案した skill を番号で実行
/cancel                             # 実行中の AI タスク/skill を中止 (引数不要)
```

### Avatar (Phase 2)

```
/avatar set <target> <preset>       # target: self, @ops-ai
/avatar list
/avatar preview
/avatar mode <compact|normal|expressive>
```

### Art

```
/art list                           # art.yaml のショートコード一覧
/art reload                         # art.yaml を再読み込み
```

### Files (Phase 1)

```
/send <file_path>                   # ルーム内 peer にファイル送信
```

> ファイル受信時は `NetMessage::TransferOffer` → 受信側が `TransferAccept`/`TransferReject` → `UserData` チャンク → `Chunk::End` のフロー。100 MB 上限 (`MAX_TRANSFER_SIZE`)。

---

## 14. 設定ファイル

パス: `~/.config/triadchat/config.toml`（初回起動時に `Config::default()` から自動生成）

```toml
# --- フラットキー (CLI フラグで上書き可能) ---
discovery_addr   = "238.255.0.1:5877"   # UDP mcast
tcp_server_port  = 0                    # 0 = ランダム
user_name        = "your-name"          # 空 = whoami::username()
terminal_bell    = true

[language]
# AI 出力言語 (プロンプトの言語指示に使用): "ja" | "en" | "zh" | "ko"
ai_output = "ja"
# UI システムメッセージ言語 (接続通知・エラー等): "ja" | "en" (zh/ko は "en" にフォールバック)
ui        = "ja"

[ai]
enabled     = true
provider    = "claude"   # claude | codex | gemini | custom
# command    = "/path/to/claude"   # claude が PATH にない場合のみ上書き
timeout_secs = 30

[security]
default_permission = "confirm-required"   # confirm-required | trusted-auto-safe | deny-remote-exec
trusted_peers      = []

[user]
avatar    = "human_default"   # preset 名
ai_avatar = "ai_default"

# [theme] は初回起動時に自動生成される Color 定義一式。通常は手動編集しない。
```

> `[network]` / `[ui]` / `[claude_code]` セクションは存在しない（旧仕様の誤記）。ネットワーク設定はフラットキー、UI 設定は `[theme]`、Claude Code 呼び出しは `[ai]` 配下に統合されている。

実装の真実の構造体（`src/config.rs`）:

```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub discovery_addr: SocketAddrV4,
    pub tcp_server_port: u16,
    pub user_name: String,
    pub terminal_bell: bool,
    pub language: LanguageConfig,
    pub ai: AiConfig,
    pub security: SecurityConfig,
    pub theme: Theme,
    #[serde(default)]
    pub user: UserConfig,
}

pub struct AiConfig {
    pub enabled: bool,
    pub provider: AiProvider,        // Claude | Codex | Gemini | Custom (serde lowercase)
    pub command: Option<String>,
    pub timeout_secs: u64,
}

pub struct SecurityConfig {
    pub default_permission: String,  // 文字列で保持、default_permission_policy() で enum 化
    pub trusted_peers: Vec<String>,
}

pub struct UserConfig {
    pub avatar: String,
    pub ai_avatar: String,
}

pub struct LanguageConfig {
    pub ai_output: String,   // "ja" | "en" | "zh" | "ko"
    pub ui: String,          // "ja" | "en"
}

impl Default for LanguageConfig {
    fn default() -> Self {
        // $LANG 環境変数から自動判定 (ja→ja, en/zh/ko/不明→en for ui)
        Self::from_lang_env_value(std::env::var("LANG").ok().as_deref())
    }
}
```

---

## 15. Transcript (Phase 1)

保存先: `~/.local/share/triadchat/transcripts/<room_id>.jsonl`

```jsonl
{"id":"msg-001","room_id":"room-001","sender_id":"takuro","sender_type":"human","text":"この関数重い","kind":"chat","timestamp":"2026-04-04T10:01:00+09:00","intent":null,"structured":null}
{"id":"msg-002","room_id":"room-001","sender_id":"ops-ai","sender_type":"ai","text":"決定: auth を service 層へ","kind":"ai","timestamp":"2026-04-04T10:03:00+09:00","intent":"decision","structured":{"todos":[{"text":"auth 抽出設計","assignee":"takuro"}],"decisions":["auth を service 層へ"],"skill_suggestions":["review-auth"]}}
```

---

## 16. ネットワーク安全設計 (Phase 1)

- デフォルト LAN 限定
- 初回接続時に `PeerIdentity` 署名を検証し、公開鍵 fingerprint を endpoint に紐づける
- 署名検証後の endpoint が `PeerInfo` の `user_name`, `node_version`, `server_port` を変更した場合は切断する
- peer 由来で state を変更する payload (`UserMessage`, `UserData`, `AiMessage`, `RoomCreate`, `RoomCreateV2`, `RoomJoin`, `SkillResult`) は署名検証済み endpoint からのみ受理する
- `trusted_peers` に含まれる peer のみスキル実行を許可
- `risk: medium|high` のスキルは明示承認が必要
- Transcript はローカル保存のみ
- TCP 通信は鍵交換 (x25519) 後 ChaCha20Poly1305 で暗号化される。鍵交換は PeerIdentity 署名検証後に開始し、x25519 公開鍵に ed25519 署名を付与して送信する。暗号化セッション確立後、チャットメッセージやスキル結果等のペイロードは `NetMessage::Secure` で暗号化転送される。未暗号化ピアとは従来の平文通信も許容する

---

## 17. エラーハンドリング

- ライブラリ層: `thiserror` で型付きエラー
- アプリ層: `anyhow::Context` で `.context("...")?`
- termchat の `util::Error` (`Box<dyn std::error::Error + Send>`) は段階的に移行
- sidecar タイムアウト (30秒) は `AiState::Failed` に遷移し TUI に表示

---

## 18. 非機能要件

| 項目 | 目標値 |
|------|--------|
| 起動時間 | 1 秒以内 |
| ピア探索 | 3 秒以内 (Phase 1) |
| sidecar タイムアウト | 30 秒 |
| AI 非利用時の CPU | termchat と同等 |
| プラットフォーム | macOS, Linux, WSL |
| Rust edition | 2021 |
| MSRV | 1.82 |
| **入力言語** | Unicode 全角対応済み (termchat 継承)。IME 未確定文字は terminal 依存 |
| **AI 出力言語** | `config.toml [language] ai_output` で設定。対応: ja / en / zh / ko |
| **UI 言語** | `config.toml [language] ui` で設定。対応: ja / en。未設定時は `$LANG` から自動判定 |

---

## 19. ディレクトリ構成 (実行時)

```
~/.config/triadchat/
├── config.toml
└── avatars/               # [Phase 2] プラグイン .so/.dylib

~/.local/share/triadchat/
├── transcripts/           # [Phase 1]
│   └── <room_id>.jsonl
└── skills_cache.json      # [Phase 1] skill registry キャッシュ

<workspace>/
└── .claude/
    ├── skills/
    │   ├── review-auth/SKILL.md
    │   └── inspect-amr/SKILL.md
    └── agents/
        └── security-reviewer.md
```

---

## 20. 実装順序と依存グラフ

依存関係:

```
[A] fork + rename + prefix変更
  └─► [B] tokio Runtime + Signal 拡張
        └─► [C] SidecarAdapter (claude -p, timeout)
              └─► [D] prompt.rs ★最優先
                    └─► [E] AiMediator + trigger
                          └─► [F] /summary /todos /decisions
                                       ↑ Phase 0 完了ライン

[B] ──► [G] NetMessage 拡張 + PeerInfo
          └─► [H] Room エンジン + LAN discovery
                └─► [I] SkillRegistry (frontmatter scan)
                      └─► [J] SkillExecutor + 承認 UI (非同期)
                            └─► [K] Transcript JSONL
                                         ↑ Phase 1 完了ライン

[F] ──► [L] AvatarPlugin trait + builtin プリセット
          └─► [M] 3ペイン TUI (peers / status)
                └─► [N] AvatarManager (libloading)
                      └─► /avatar コマンド
                                   ↑ Phase 2 完了ライン
```

### Step A: fork + リネーム + コマンドプレフィックス変更
- `Cargo.toml`: `name = "triadchat"`
- `src/commands/mod.rs`: `COMMAND_PREFIX = "/"` (1行変更)
- Config パス: `~/.config/triadchat/config.toml`
- **リスク:** なし (機械的変更)
- **検証:** `cargo build` が通ること

### Step B: tokio Runtime + Signal 拡張
- `Cargo.toml` に `tokio = { features = ["full"] }` 追加
- `Signal` に `AiResponse`, `SkillDone` 追加
- `Runtime::new()` を `Application::new()` 内で生成、`Handle` を保持
- **リスク:** message-io スレッドと tokio の競合
- **対策:** `#[tokio::main]` は使わず `Runtime::new()` + `Handle::spawn()` で分離
- **検証:** ダミー `Signal::AiResponse` を spawn して受信できること

### Step C: SidecarAdapter
- 起動時 `which::which("claude")` チェック
- `tokio::time::timeout(30s, Command::new("claude").arg("-p")...)`
- stdout が空なら `Err`
- **リスク:** `claude -p` のレスポンス形式変化
- **対策:** stdout を raw string で受け取り、パースは `prompt.rs` に委譲
- **検証:** `claude` を `echo` に差し替えた mock で unit test

### Step D: prompt.rs ★ Phase 0 の最優先事項
- `summary_prompt()` / `intervene_prompt()` / `todo_prompt()` を集約
- 出力フォーマットを厳密に定義 (パーサーと対になる)
- **リスク:** LLM 出力が不安定
- **対策:**
  1. フォーマット指示を明示 ("出力は上記フォーマットのみ")
  2. パーサーを defensive に (想定外フォーマットは raw text として扱う)
  3. `tests/prompt_quality.rs` でゴールデンテスト
- **プロンプト会話長上限:** 直近 100 行 (約 5,000 文字)。古い会話は精度寄与が低い
- **検証:** 10パターンのサンプル会話で TODO/決定事項が正確に出ること

### Step E-F: AiMediator + コマンド実装
- `AiMode::Clerk` の `should_intervene()` 実装
- `AbortHandle` を `State` に保持し、`ai_thinking` のリークを防ぐ
- `/summary` `/todos` `/decisions` → `SidecarAdapter::ask(prompt)` → parse → 表示
- **検証:** Phase 0 受け入れ基準全項目

### Step G-H: NetMessage + Room エンジン (Phase 1)
- `NetMessage` に `PeerInfo`, `RoomCreate`, `RoomJoin` を追加
- `HelloUser` との後方互換: 旧 variant を受け取れるよう variant 追加のみ (削除・並べ替え禁止)
- **リスク:** bincode の enum variant 順序に依存
- **対策:** 新 variant は末尾に追加のみ
- **検証:** 2台で起動して peer discovery が 3秒以内に成立すること

### Step I-J: SkillRegistry + Executor (Phase 1)
- `.claude/skills/` スキャン + frontmatter パース (エラーは warn + skip)
- `skills_cache.json` に mtime キャッシュ
- `state.pending_confirmation: Option<SkillMeta>` (同時1件)
- 非同期実行: skill タイムアウトは 60秒 (AI より長め)
- **検証:** `review-auth` の confirm → 実行 → `SkillDone` の状態遷移が通ること

### Step K: Transcript (Phase 1)
- append-only JSONL、`Drop` で flush
- write 失敗は warn ログのみ (チャットを止めない)
- **検証:** セッション後に `.jsonl` のラウンドトリップが正しいこと

### Step L-N: Avatar + 3ペイン TUI (Phase 2)
- `AvatarPlugin` trait + builtin プリセット
- tui `Layout::horizontal` 3分割
- `libloading` で `.so/.dylib` ロード
- **リスク:** FFI vtable の ABI 安定性
- **対策:** `#[repr(C)]` + `extern "C"` + vtable にバージョン番号を含める
- **検証:** `robot_guardian` が state 変化で表示が変わること

---

## 21. 受け入れ基準とテスト計画

### Phase 0

受け入れ基準:
- [ ] `cargo build` が通る
- [ ] `/summary` で直近会話の要約が出る
- [ ] `/todos` で担当者付き TODO が出る (担当者が読み取れる場合)
- [ ] `/decisions` で決定事項が出る
- [ ] AI が thinking 中にヘッダーが `[ops-ai: thinking...]` に変わる
- [ ] 30秒タイムアウトで `[ops-ai: failed]` 表示になる
- [ ] `claude` コマンドが存在しない場合、起動時にわかりやすいエラーが出る

テスト:
```
tests/
├── sidecar_mock.rs      # claude を echo で差し替えた mock テスト
├── prompt_quality.rs    # ゴールデンテスト (10会話サンプル)
│   fixtures/
│   ├── dev_review.txt
│   ├── ops_incident.txt
│   └── expected/*.json  # 期待出力
├── trigger_test.rs      # should_intervene() 全条件のユニットテスト
└── classifier_test.rs   # MessageClass 分類のユニットテスト
```

### Phase 1

受け入れ基準:
- [ ] 2台で起動し peer discovery が 3秒以内に成立する
- [ ] `/room create @user1` でルームが作られ AI が付与される
- [ ] `/skills` で `.claude/skills/` の一覧が出る
- [ ] `/skill review-auth` で承認ダイアログが出る
- [ ] 承認後に非同期で実行され、実行中は `[acting]` 表示になる
- [ ] 実行結果が room に投稿される
- [ ] `risk:medium` スキルは `y/n` 確認なしには実行されない
- [ ] transcript に会話・AI・skill 結果が時系列保存される
- [ ] bincode の後方互換: 旧 `HelloUser` のみのノードと接続できる

テスト:
```
tests/
├── network_integration.rs  # 2プロセス起動してピア発見確認
├── skill_registry.rs       # .claude/skills/ スキャンのユニットテスト
│   fixtures/
│   └── .claude/skills/review-auth/SKILL.md  # テスト用フィクスチャ
├── skill_executor.rs       # confirm → 実行 → SkillDone の状態遷移テスト
└── transcript.rs           # JSONL ラウンドトリップテスト
```

### Phase 2

受け入れ基準:
- [ ] ASCII avatar が AI state (idle/thinking/acting/failed) に応じて変化する
- [ ] `/avatar set @ops-ai robot_guardian` でアバターが変わる
- [ ] `~/.config/triadchat/avatars/` のプラグインが `/avatar list` に出る
- [ ] 3ペイン TUI で peers / status が表示される
- [ ] TUI が 80列未満に縮小されたとき avatar が compact に自動縮退する

テスト:
```
tests/
├── avatar_builtin.rs   # 全プリセット × 全 state × 全 size の組み合わせ
├── avatar_plugin.rs    # テスト用 dylib をロードして render() が返ること
└── ui_layout.rs        # 3ペインのサイズ計算 (termchat の ui-test feature 活用)
```

---

## 22. アーキテクチャ上の決定事項

実装前に確定しておく判断:

| 項目 | 決定 | 理由 |
|------|------|------|
| tokio と message-io の統合 | `Runtime::new()` + `Handle::spawn()` | `#[tokio::main]` は message-io の内部スレッドと競合する可能性があるため手動生成 |
| プロンプトの会話長上限 | 直近 100 行 (約 5,000 文字) で切り詰め | 古い会話は精度寄与が低い。`claude -p` の stdin 制限も考慮 |
| `pending_confirmation` の型 | `Option<SkillMeta>` (1件のみ) | v1 は同時1件。複数同時実行は Phase 2 以降 |
| NetMessage シリアライズ | bincode 維持 (termchat 互換) | enum variant は末尾追加のみ。削除・並べ替えは後方互換を壊す |
| skill タイムアウト | 60秒 (AI は 30秒) | skill は重い処理を含む可能性がある |
| Transcript write 失敗 | warn ログのみ、チャットを止めない | ストレージ障害でチャット機能が止まるのは不可 |
| avatar FFI ABI | `#[repr(C)]` vtable + バージョン番号フィールド | dylib の ABI 安定性を保証するため |
