# live-scratch

sb3ファイルをテキストエディタで編集し、変更を即座にブラウザ上のScratchエディタに反映するツール。

<video src="demo.mp4" controls></video>

## Scratch x Vibe Coding

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

## セットアップ

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
                                    ↓ chokidar監視
                            [Node.js Server]
                            (Express + WebSocket + chokidar)
                                    ↓ WebSocket (ArrayBuffer)
                            [Browser: Scratch GUI + live-reload.js]
                                    ↓ vm.loadProject(arrayBuffer)
                            [Scratch エディタが即座に更新]
```

## 注意事項

- `project.json` にJSON構文エラーがある場合、ターミナルにエラーが表示され送信はスキップされる
- `vm.loadProject()` はプロジェクト全体をリロードするため、実行中のスクリプト・変数の実行時値・クローンは初期化される
