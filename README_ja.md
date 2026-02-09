# Live Scratch

[English](README.md)

プロジェクトファイルとScratchエディタを双方向にライブ同期するデスクトップアプリ（macOS）。テキストエディタでsb3プロジェクトファイルを編集すると即座に反映され、Scratchエディタでの変更も自動的に書き戻されます。

## Scratch x Vibe Coding

[![Demo](https://img.youtube.com/vi/uoXJ0N3IdK0/maxresdefault.jpg)](https://www.youtube.com/watch?v=uoXJ0N3IdK0)

live-scratch は `~/Documents/Live Scratch/` に `project.json` をプレーンなJSONとして公開します。つまり **AIコーディングエージェントで直接 Scratch プロジェクトを編集できます。**

[Claude Code](https://github.com/anthropics/claude-code)、[Codex](https://github.com/openai/codex)、[Gemini CLI](https://github.com/google-gemini/gemini-cli) などのAIエージェントに「ネコを犬に変えて」「ゲームオーバー表示を追加して」と自然言語で指示するだけで、Scratchプロジェクトがリアルタイムに変化していきます。

```
あなた: 「ネコを犬に変えて」
  ↓ AIが ~/Documents/Live Scratch/project.json を編集
  ↓ live-scratch が変更を検知
  ↓ Scratchエディタに即座に反映
あなた: 結果を見ながら次の指示へ
```

ブロックを手でドラッグする代わりに、会話でScratchプログラミング。結果はリアルタイムに確認できます。

## 対応環境

- macOS 10.15 (Catalina) 以降
- Apple Silicon (M1/M2/M3/M4) および Intel

## ダウンロード

最新の `.dmg` を [GitHub Releases](https://github.com/champierre/live-scratch/releases/latest) からダウンロードできます。

> このアプリはコード署名されていません。初回起動時に macOS がブロックします。アプリを右クリックして「開く」を選択し、ダイアログで「開く」をクリックしてください。

## 使い方

1. Live Scratch アプリを開く
2. デフォルトプロジェクト（Scratchの初期状態）が `~/Documents/Live Scratch/` に作成される
3. `project.json` やアセットファイルをテキストエディタやAIエージェントで編集
4. 変更が即座にScratchエディタに反映される
5. Scratchエディタでの変更（ブロック追加、スプライト変更、コスチューム・サウンド追加など）は自動的に `~/Documents/Live Scratch/` に保存される

### メニュー

- **File > Open SB3...** (`Cmd+O`) — 既存の `.sb3` ファイルを読み込む
- **File > Export SB3...** (`Cmd+S`) — 現在のプロジェクトを `.sb3` として保存
- **File > Show Workspace in Finder** (`Cmd+Shift+O`) — `~/Documents/Live Scratch/` を Finder で開く

## セットアップ（ソースからビルド）

前提条件: [Node.js](https://nodejs.org/) と [Rust](https://www.rust-lang.org/tools/install)

```bash
git clone https://github.com/champierre/live-scratch.git
cd live-scratch
npm install
```

`npm install` は `setup.sh` を通じて自動で以下を実行します：

1. [scratch-editor](https://github.com/scratchfoundation/scratch-editor) をバージョン固定（`81d16ac24`）でクローン
2. `window.vm` 公開パッチと TypeScript 型宣言パッチを適用
3. `npm install` と `scratch-gui` のビルド

開発モードで実行：

```bash
npm run tauri:dev
```

## アーキテクチャ

```
[テキストエディタ / AIエージェント]
    ↕ ~/Documents/Live Scratch/ のファイルを編集
[Rust バックエンド (Tauri v2)]
    workspace.rs  — SB3 ビルド/展開
    watcher.rs    — ファイル監視 (notify クレート)
    commands.rs   — Tauri IPC コマンド
    lib.rs        — アプリ初期化、メニュー
    ↕ Tauri IPC (window.__TAURI__)
[Scratch GUI + live-reload.js (WebView)]
    ↕ vm.loadProject / vm.saveProjectSb3
[Scratch エディタ]
```

- **ワークスペース → エディタ**: ファイルウォッチャーが変更を検知し、SB3をビルドしてTauriイベント経由でフロントエンドに送信
- **エディタ → ワークスペース**: `PROJECT_CHANGED` イベントをリッスンし、デバウンス（1秒）後にSB3をRustバックエンドに送信、ワークスペースに展開
- **ループ防止**: フロントエンド・バックエンド両方でフラグとタイムアウトによる無限同期ループ防止機構を実装

## 注意事項

- `project.json` にJSON構文エラーがある場合、更新はスキップされエラーがログに記録される
- `vm.loadProject()` はプロジェクト全体をリロードするため、実行中のスクリプト・変数の実行時値・クローンは初期化される
