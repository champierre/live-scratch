# Live Scratch

[English](README.md)

`workspace/` ファイルとブラウザ上のScratchエディタを双方向にライブ同期するツール。テキストエディタでsb3プロジェクトファイルを編集すると即座にブラウザに反映され、Scratchエディタでの変更も自動的に `workspace/` に書き戻されます。

## Scratch x Vibe Coding

![Demo](demo.gif)

live-scratch は `workspace/project.json` をプレーンなJSONとして公開します。つまり **AIコーディングエージェントで直接 Scratch プロジェクトを編集できます。**

[Claude Code](https://github.com/anthropics/claude-code)、[Codex](https://github.com/openai/codex)、[Gemini CLI](https://github.com/google-gemini/gemini-cli) などのターミナルAIエージェントに「ネコを犬に変えて」「ゲームオーバー表示を追加して」と自然言語で指示するだけで、Scratchプロジェクトがリアルタイムに変化していきます。

```
あなた: 「ネコを犬に変えて」
  ↓ AIが workspace/project.json を編集
  ↓ live-scratch が変更を検知
  ↓ ブラウザのScratchエディタに即座に反映
あなた: 結果を見ながら次の指示へ
```

ブロックを手でドラッグする代わりに、会話でScratchプログラミング。結果はリアルタイムにブラウザで確認できます。

## ダウンロード

最新の `.dmg` を [GitHub Releases](https://github.com/champierre/live-scratch/releases/latest) からダウンロードできます。

> このアプリはコード署名されていません。開こうとすると「"Live Scratch.app"は壊れているため開けません」と表示されます。以下のコマンドを実行してから開いてください：
>
> ```bash
> xattr -cr "/Applications/Live Scratch.app"
> ```

## セットアップ（ソースからビルド）

```bash
git clone https://github.com/champierre/live-scratch.git
cd live-scratch
npm install
```

`npm install` は依存パッケージのインストール後、自動で以下を実行します：

1. [scratch-editor](https://github.com/scratchfoundation/scratch-editor) をバージョン固定（`81d16ac24`）で clone
2. `window.vm` 公開パッチと TypeScript 型宣言パッチを適用
3. `npm install` と `scratch-gui` のビルド

## 使い方

```bash
npm start
```

1. デフォルトプロジェクト（Scratchの初期状態）が `workspace/` に展開される
2. ブラウザが自動でScratchエディタを開く
3. `workspace/project.json` やアセットファイルをテキストエディタで編集・保存
4. 変更が即座にブラウザのScratchエディタに反映される
5. Scratchエディタでの変更（ブロック追加、スプライト変更、コスチューム・サウンド追加など）は自動的に `workspace/` に保存される

画面右上の丸いインジケーターで接続状態を確認できる（緑=接続中、赤=切断）。

既存のsb3ファイルから始める場合：

```bash
npm start -- myproject.sb3
```

ポートを変更する場合は `--port` オプションを指定：

```bash
npm start -- --port 8080
```

## アーキテクチャ

```
[テキストエディタ] → 編集 → [workspace/project.json + assets]
                                    ↕ 双方向同期
                            [Node.js Server]
                            (Express + WebSocket + chokidar)
                                    ↕ WebSocket (ArrayBuffer)
                            [Browser: Scratch GUI + live-reload.js]
                                    ↕ vm.loadProject / vm.saveProjectSb3
                            [Scratch エディタ]
```

- **workspace → ブラウザ**: chokidarがファイル変更を監視し、sb3をビルドしてWebSocket経由で送信
- **ブラウザ → workspace**: `PROJECT_CHANGED` イベントをリッスンし、デバウンス（1秒）後にsb3をサーバーに送信、サーバーが `workspace/` に展開
- **ループ防止**: クライアント・サーバー両方でフラグとタイムアウトによる無限同期ループ防止機構を実装
- **複数タブ対応**: あるクライアントからの変更は他の接続中クライアントすべてにブロードキャスト

## 注意事項

- `project.json` にJSON構文エラーがある場合、ターミナルにエラーが表示され送信はスキップされる
- `vm.loadProject()` はプロジェクト全体をリロードするため、実行中のスクリプト・変数の実行時値・クローンは初期化される
