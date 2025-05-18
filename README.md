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

* **Kanon (フロントエンド):** ユーザーインターフェースを提供し、GUI で作成された仕様をこのバックエンドに送信します。合成結果を受け取り、エディタに表示します。Kanonは豊富なサンプルデータ構造（連結リスト、ツリー、グラフなど）を含み、様々な操作例を提供します。
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

## サンプルデータ構造 (Sample Data Structures)

Kanonフロントエンドは、以下のカテゴリの豊富なサンプルデータ構造を提供しています：

1. **基本データ構造 (basics):**
   - 単方向連結リスト (singular-linked-list.js)
   - 双方向連結リスト (doubly-linked-list.js)
   - AVL木 (AVL.js)
   - 二分探索木 (binary-search-tree.js)
   - 挿入ソート (linked-list-insertSort.js)

2. **高度なレイアウト用データ構造 (for_FIFA_layout):**
   - グラフ (graph.js)
   - B+木 (B+_tree.js)
   - フィボナッチヒープ (fibonacci_heap.js)
   - スキップリスト (skip_list.js)
   - 赤黒木 (red_black_tree.js)
   - 三分木 (ternary_tree.js)
   - 隣接行列 (Adjacencey_Matrix.js)
   - ダイクストラ最短経路 (dijkstra_shortest_path.js)
   - ラムダ計算 (lambda-calculus.js)
   - その他、複合データ構造の例

これらのサンプルは、データ構造に対する操作の動作を視覚的に理解し、自動合成のための仕様を定義する際の参考となります。

## ディレクトリ構成とモジュールの役割 (Directory Structure and Roles)
```
refsyn/
├── src/
│   ├── main.rs          # アプリケーションのエントリーポイント、HTTPリクエスト処理、共通パターン抽出
│   ├── server.rs        # HTTPサーバーロジック（warp）、ルーティング定義
│   ├── ast.rs           # プログラムの内部表現 (AST) の定義
│   ├── parser.rs        # KanonからのJSONをASTに変換するパーサー
│   ├── env.rs           # 変数スコープと外部参照を管理する環境 (MemoEnv)
│   ├── models.rs        # APIリクエスト/レスポンスのデータ構造定義 (serde)
│   └── error.rs         # プロジェクト全体で使用するエラー型定義 (現在は未使用の可能性あり)
│
├── Kanon/               # Git submodule: Kanonフロントエンド
├── Cargo.toml           # Rustプロジェクトの定義ファイル
├── Cargo.lock           # 依存関係のロックファイル
├── README.md            # このファイル
└── target/              # ビルド成果物 (gitignore対象)
```

* **`src/main.rs`:** アプリケーションのエントリーポイント。HTTPリクエストを処理し、`parser.rs` を使って入力をASTに変換後、複数のASTから共通パターンと差分（ホール）を抽出するコアロジックを含みます。最終的に `server.rs` を介してレスポンスを返します。
* **`src/server.rs`:** [Warp](https://github.com/seanmonstar/warp) を使用してHTTPサーバーをセットアップし、ルーティング (例: `/synthesize`) を定義します。リクエストの受付とレスポンスの返却を担当します。
* **`src/ast.rs`:** プログラムの内部表現であるAST (Abstract Syntax Tree) のデータ構造 (`Program`, `Stmt`, `Expr`, `Lhs` など) を定義します。
* **`src/parser.rs`:** Kanonフロントエンドから送信されるJSON形式の操作列 (`Vec<Operation>`) を解析し、`ast.rs` で定義された `Program` 構造体に変換します。この際、`env.rs` の `MemoEnv` を利用して変数解決やスコープ管理を行います。
* **`src/env.rs`:** `MemoEnv` 構造体を定義し、複数のメソッド呼び出しにまたがる変数のスコープ管理、IDと名前のマッピング、外部から参照される際のアクセスパスの解決など、環境関連の機能を提供します。
* **`src/models.rs`:** `serde` を利用して、APIで送受信されるJSONデータに対応するRustの構造体 (`SynthesisRequest`, `SynthesisResponse`, `MethodCallOperation` など) を定義します。
* **`src/error.rs`:** (現状、積極的に使用されていない可能性があります) プロジェクト全体で使用するカスタムエラー型を定義するためのファイルです。
* **`Kanon/`:** Git submoduleとして管理されているKanonフロントエンドのコードが含まれます。
* **`target/`:** Rustのビルドプロセスによって生成されるファイルが格納されるディレクトリで、`.gitignore` によってバージョン管理から除外されています。

## ワークフロー概要 (Workflow)

1.  **仕様受信と解析:**
    *   Kanonフロントエンドは、ユーザーがGUIで定義した複数のメソッド呼び出しに関する情報（操作列、レシーバーオブジェクト、メソッド名などを含む `MethodCallOperation` のリスト）を、RefSynバックエンドの `/synthesize` エンドポイントに送信します。
    *   `server.rs` がリクエストを受け付け、`main.rs` の `handle_synthesis` 関数に渡します。
    *   `handle_synthesis` 関数は、各 `MethodCallOperation` に対して以下の処理を行います:
        *   新しい `MemoEnv` インスタンス（`env.rs`）を作成し、現在のメソッド呼び出しのスコープとレシーバーオブジェクト (`this`) を設定します。
        *   `parser.rs` の `parse_operations` 関数を呼び出し、操作列を `ast::Program` に変換します。この際、`MemoEnv` を使用して変数の解決やスコープ内での登録、外部参照の可能性のある割り当ての記録を行います。
        *   変換された `ast::Program` と、その解析に使用された `MemoEnv` の状態を保存します。

2.  **共通パターン抽出:**
    *   全ての `MethodCallOperation` の解析が完了した後、`main.rs` 内の `find_common_pattern_and_holes` 関数が、生成された `ast::Program` のリストと、各解析に対応する `MemoEnv` のリストを入力として受け取ります。
    *   この関数は、ASTの構造を比較し、共通する部分と異なる部分（ホールとして表現）を特定します。
        *   （今後の拡張）`MemoEnv` に記録された外部参照情報 (`get_access_path`) を利用して、異なるスコープで異なる名前を持つが構造的に同じ変数を識別し、より正確なホール生成を目指します。例えば、`lst.append(0)` での `lst` と `lst.append(3)` での `lst` が、たとえ内部IDが異なっても、どちらも呼び出し元の同じ変数 `this.listField` を指している場合、それらを同一視してホール化を避ける、またはホール化する場合でもその関連性を示す情報を付加します。
        *   リテラル値の違い（例: `0` と `3`）もホールとして識別されます。

3.  **応答:**
    *   抽出された共通パターン（ホールを含むASTとして表現）と、各ホールの具体的な値のリスト（どのメソッド呼び出しでどの値が使われたか）を含む `SynthesisResponse` を生成します。
    *   このレスポンスをJSON形式でKanonフロントエンドに返却します。

4.  **(将来展望) PBEシンセサイザとの連携:**
    *   現状のコアロジックは共通パターンの抽出とホール化に焦点を当てています。
    *   将来的には、`find_common_pattern_and_holes` で特定されたホール（特に複雑な式やロジックが入りうる箇所）を埋めるために、外部のPBE (Programming by Example) エンジン（例: Escher-Scala）と連携する機能が追加される可能性があります。
    *   その場合、ホールに対応するPBE問題（入出力例）を生成し、PBEエンジンに送信、得られた解をホールに埋め込む、というステップが追加されます。

## 外部依存関係 (External Dependencies)

* **Kanon:** RefSyn のフロントエンドとして機能します。
* **Escher-Scala (改変版):** 実際の PBE 合成を行うバックエンドサービスです。
