<!-- i18n: language-switcher -->
[English](README.md) | [日本語](README.ja.md)

# Migration Studio

データベースエンジン間のテーブル移行を計画する、[Irodori Table](https://github.com/irodori-table/irodori-table) の拡張機能です。拡張機能ストアからインストールすると、**Tools → Migration Studio** かコマンドパレットから開けます。

移行元と移行先のテーブルから、ソース SQL、ターゲット SQL、データ差分、Runbook を生成します。組み合わせごとの検証ゲートとエンジン注記も表示します。生成するのはテキストだけです。データベースには接続せず、SQL の実行はご自身で行います。

計画の生成は [`irodori-migration`](https://github.com/irodori-table/irodori-migration) クレートが担当します。この拡張機能は、本体のフォームをクレートの入力に変換し、計画を出力タブに変換します。

## 仕組み

この拡張機能は、*ツール* を提供するネイティブモジュールです。`irodori.extension.json` の `contributes.tools` にフォームを記述し、本体がそれを描画するので、拡張機能のコードはアプリの webview では動きません。入力を変更するたびに `tool.run` が呼ばれ(処理は `src/driver.rs`)、その応答が表示するタブになります。本体はモジュールを別プロセスで動かすので、ここで異常終了してもアプリは落ちません。

## 開発

```sh
make check    # fmt、clippy、テスト
make package  # リリースビルドと dist/irodori-extension-migration.tar.gz の作成
```

## ライセンス

0BSD または MIT のいずれかを選べます。
