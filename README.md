# RefSyn Backend

## 概要 (Introduction)

RefSyn は、データ構造に対する操作の具体例（メソッド呼び出し前後のグラフ状態）から、その操作を行うメソッドの実装を自動合成するシステムです。このリポジトリは、RefSyn の **バックエンドサーバー** を Rust で実装したものです。

主な役割は以下の通りです:

* **Kanon フロントエンドからのリクエスト受信:** ユーザーが Kanon 上の GUI で指定したメソッド仕様（操作列、実行前状態など）を受け取ります。
* **仕様の処理:** 受け取った仕様を解析し、内部的なプログラム表現 (AST) に変換します。
* **共通構造の抽出:** 複数の仕様から共通するプログラム構造を特定し、可変部分（差分）を識別します。
* **PBE シンセサイザとの連携:** 可変部分の合成のために、外部の PBE (Programming by Example) エンジンである **Escher-Scala** (改変版) と通信します。
* **結果の統合と返却:** Escher-Scala から得られた合成結果を共通構造に統合し、最終的なメソッドコードを Kanon に返します。

## アーキテクチャ概要 (Architecture Overview)

このバックエンドは、[Warp](https://github.com/seanmonstar/warp) フレームワークを使用した Rust 製の Web サーバーとして構築されています。以下の主要なコンポーネントと連携します:

* **Kanon (フロントエンド):** ユーザーインターフェースを提供し、GUI で作成された仕様をこのバックエンドに送信します。合成結果を受け取り、エディタに表示します。
* **Escher-Scala (PBE バックエンド):** 実際に PBE 合成を行う独立したサービス（Scala で実装）。この RefSyn バックエンドは、Escher-Scala に対して合成タスク（`tests.json` 形式で記述）を依頼し、結果を受け取ります。（連携は HTTP 経由を想定）。

コアロジックは以下のステップを含みます:

1.  Kanon からの GUI 操作列のパース。
2.  内部 AST (Abstract Syntax Tree) への変換。
3.  複数の AST を比較することによる共通構造と差分の発見。
4.  差分部分に対する PBE 問題（Escher-Scala 向け `tests.json`）の生成。
5.  Escher-Scala サービスとの通信。
6.  合成結果の統合。

## 差分解析のフロー (Difference-Based Approach)

ユーザーは Kanon 上でメソッド呼び出し (例: `lst.append(0)`) に対して
「メソッド呼び出し前 -> メソッド呼び出し後」のグラフ操作をGUIで指定し、
操作列を本バックエンドに送信します。その後、バックエンドは以下を行います:

1. 受信した操作列 (メソッド呼び出し前後の差分) を内部ASTに変換する。
2. 複数の操作列から共通構造と可変部分を抽出する。
3. 可変部分を外部合成器 (Escher-Scala) に渡せる形式に準備する。

このようにして、複数の仕様からメソッド呼び出しの共通パターンを見つけ、
差分となる部分のみを合成用にエンコードする仕組みを提供します。

## ディレクトリ構成とモジュールの役割 (Directory Structure and Roles)
```
refsyn/src/
├── main.rs          # アプリケーションのエントリーポイント、サーバーの起動
├── server.rs        # HTTPサーバーロジック（warp）、ルーティング、リクエスト/レスポンス処理
├── error.rs         # プロジェクト全体で使用するエラー型定義
│
├── ast.rs           # プログラムの内部表現 (AST) の定義 (Program, Statement, Lhs, Exp など)
│
├── parser.rs        # Kanonからの operations JSON を AST に変換するパーサー
│
├── synthesizer/
│   ├── mod.rs       # 合成プロセス全体を管理するモジュール
│   ├── common.rs    # 複数のASTから共通構造と差分を抽出するロジック
│   └── pbe.rs       # PBE (Escher-Scala) 関連の処理
│       ├── mod.rs
│       ├── encoder.rs # プログラム状態をEscher向け形式にエンコード (RefSyn Step 5)
│       ├── examples.rs# Escher向け入出力例 (tests.json 形式) 生成 (RefSyn Step 6)
│       └── client.rs  # Escher-Scalaサービスと通信するクライアント (HTTP想定)
│   └── integrator.rs  # Escherの合成結果を共通ASTに統合 (RefSyn Step 9)
│
└── models.rs        # APIリクエスト/レスポンスのデータ構造定義 (serde)
```
# (Operation, SynthesisRequest, SynthesisResponse など)
* **`main.rs`:** アプリケーションのエントリーポイント。`server.rs` の起動関数を呼び出します。
* **`server.rs`:** Warp を使用して HTTP エンドポイント (例: `/synthesize`) を定義し、リクエストを処理します。`models.rs` で定義された構造体を使って JSON をデシリアライズ/シリアライズし、`parser.rs` や `synthesizer` モジュールの関数を呼び出します。
* **`models.rs`:** `serde` を利用して、API で送受信される JSON データに対応する Rust の構造体を定義します。Kanon から送られる `Operation` の詳細構造もここで定義します。
* **`error.rs`:** アプリケーション固有のエラー型を定義し、エラーハンドリングを統一します。
* **`ast.rs`:** コードスニペットを表現するための内部的な AST データ構造 (`Program`, `Statement`, `Lhs`, `Exp` など) を定義します。これらの構造はプログラムの解析と比較の基礎となります。
* **`parser.rs`:** Kanon フロントエンドから送られてくる `Vec<Operation>` (ユーザーの GUI 操作列) を解析し、`ast::Program` 構造体に変換するロジックを担当します。
* **`synthesizer/`:** コアな合成ロジックを含むモジュールです。
    * **`mod.rs`:** 合成プロセス全体を調整します。
    * **`common.rs`:** 複数の `ast::Program` を入力として受け取り、それらに共通する構造を持つ新しい AST (差分はプレースホルダーで表現) と、差分の詳細情報リストを生成するアルゴリズムを実装します。
    * **`pbe/`:** 外部の Escher-Scala PBE エンジンとの連携を担当します。
        * **`encoder.rs`:** PBE エンジンに入力する前に、プログラムの状態（グラフなど）や引数を Escher が理解できる形式（例: `List[Int]`）にエンコードします。
        * **`examples.rs`:** エンコードされた状態と各仕様における期待される値から、Escher-Scala に渡す具体的な入出力例 (`tests.json` の内容) を生成します。
        * **`client.rs`:** Escher-Scala サービスに対してネットワークリクエスト（HTTP POST など）を送信し、合成結果を受信するクライアントロジックを実装します。
    * **`integrator.rs`:** Escher-Scala から返された合成結果（差分部分のコード）を、`common.rs` で生成された共通構造 AST のプレースホルダー部分に埋め込み、最終的なメソッドコードを完成させます。
* **`storage.rs` (オプション):** 仕様を一つずつ受信し、後でまとめて合成する場合に、受信した仕様 (AST と関連情報) を一時的に保存する機能を提供します (例: インメモリの `HashMap`)。

## ワークフロー概要 (Workflow)

1.  **仕様受信:** Kanon はユーザーが定義したメソッド仕様 (操作列、コンテキスト情報、操作前グラフ状態など) を RefSyn バックエンド (このサーバー) のエンドポイント (例: `/add_specification` または `/synthesize`) に送信します。
2.  **解析と保存 (オプション):** RefSyn バックエンドは受信した操作列を `parser.rs` で `ast::Program` に変換し、必要に応じて `storage.rs` で一時保存します。
3.  **合成トリガー:** Kanon から合成開始リクエストが `/synthesize` エンドポイントに送信されます (対象メソッドを指定)。
4.  **共通化:** RefSyn バックエンドは、対象メソッドに関する保存済みの AST を取得し、`synthesizer::common.rs` を使って共通構造 AST と差分リストを生成します。
5.  **PBE 問題生成:** 差分リストと各仕様の操作前状態に基づき、`synthesizer::pbe::encoder.rs` と `synthesizer::pbe::examples.rs` を使って、各差分に対応する PBE 問題 (`tests.json` 形式のデータ) を生成します。
6.  **外部合成器呼び出し:** `synthesizer::pbe::client.rs` が、生成された PBE 問題を Escher-Scala サービスに送信します。
7.  **結果受信:** Escher-Scala は PBE 合成を実行し、結果 (差分部分のコード) を RefSyn バックエンドに返します。
8.  **結果統合:** `synthesizer::integrator.rs` が、受信した差分コードを共通構造 AST に埋め込み、最終的なメソッドコードを生成します。
9.  **応答:** RefSyn バックエンドは、完成したメソッドコードを Kanon に返します。


## 外部依存関係 (External Dependencies)

* **Kanon:** RefSyn のフロントエンドとして機能します。
* **Escher-Scala (改変版):** 実際の PBE 合成を行うバックエンドサービスです。
