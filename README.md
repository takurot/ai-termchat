# ai-termchat (triadchat)

> Terminal chat with a built-in AI clerk — no server, no GUI, just your LAN and a terminal.

**ai-termchat** は [termchat](https://github.com/lemunozm/termchat) を fork し、「人間 + AI」または「人間A・人間B・AI」の3者会話型オペレーション端末へ拡張するプロジェクトです。

ターミナルから出ずに会話するだけで、TODO・決定事項が自動で構造化され、Claude Code skills に接続できます。

---

## コアコンセプト

```
10:01 takuro: この関数、責務が多すぎる
10:02 tanaka: 認証周りを切り出したい
10:03 ops-ai ✦ 決定: auth を service 層へ分離 (既存 IF 維持)
               TODO: takuro — auth 抽出設計
                     tanaka — 回帰確認
               提案: [1] /skill review-auth
```

会話するだけで AI clerk が構造化してくれる。コマンドを覚えなくても動く。

---

## 特徴

- **サーバ不要** — LAN multicast で自動ピア発見、TCP 直結
- **TUI** — ターミナルのみで完結
- **AI clerk** — `/summary` `/todos` `/decisions` で会話を即座に構造化
- **Claude Code skills 連携** — `/skill <name>` で `.claude/skills/` のスキルを実行
- **言語設定** — `config.toml` で AI 出力言語・UI 言語を設定 (ja/en/zh/ko)
- **Avatar plugin** — ASCII アバターをプラグインで差し替え可能 (Phase 2)

---

## ステータス

現在は設計フェーズです。実装は以下のフェーズで進めます。

| フェーズ | スコープ | ステータス |
|----------|----------|-----------|
| **Phase 0** | 1人+AI。`/summary` `/todos` `/decisions` | 📋 設計中 |
| **Phase 1** | LAN 2台。2人+AI の3者 Room。`/skill` 実行 | 📋 未着手 |
| **Phase 2** | ASCII avatar プラグイン + 3ペイン TUI | 📋 未着手 |

---

## ドキュメント

| ファイル | 内容 |
|----------|------|
| [docs/IDEA.md](docs/IDEA.md) | プロダクトアイデア・コンセプト |
| [docs/SPEC.md](docs/SPEC.md) | 機能仕様書 (v0.3) |
| [docs/PLAN.md](docs/PLAN.md) | PR 単位の実装計画 |

---

## 技術スタック

- **言語:** Rust (edition 2021, MSRV 1.75)
- **ネットワーク:** [message-io](https://github.com/lemunozm/message-io) (UDP multicast + TCP)
- **TUI:** [tui-rs](https://github.com/fdehau/tui-rs) + crossterm
- **AI:** Claude Code sidecar (`claude -p`)
- **設定:** TOML

---

## ベース

termchat v1.3.1 — [github.com/lemunozm/termchat](https://github.com/lemunozm/termchat)

---

## ライセンス

Apache-2.0
