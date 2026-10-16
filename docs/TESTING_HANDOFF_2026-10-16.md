RefSyn のテスト調査と拡充に関する引き継ぎ資料（2026-10-16）

この資料は、共著者からの「意味のあるテストが足りない」という指摘を受けた調査と、その後に依頼された5メソッドの比較テスト拡充を引き継ぐためのもの。
共著者から、意味のあるテストが足りていないとの指摘を受けました。
コードを全部見るのは苦しい。最低限、各モジュールでのテストがこうであるとか検証可能で納得のいくものを出さないといけない。
例えば、合成として出してきたものが真に仕様を満たすのかエディタに反映させる前に検証して満たさなかったら合成できなかったと出すとか、特定のプログラムについては合成解と望ましいプログラムの振る舞いが一致しているか(入出力例を10-20与えて一致するか確認)とか、他にもした方がいいと言われた。あとはCLIで実際の操作ではなくある程度自動的にプログラムで検証可能な環境を整えておくとか、ランダムな操作を持っておいて正規化したときに一致するのか(これは関係ないかも？)とか、実際に操作しないでもいいような環境を整えておいた方がいいとも言われた。
まず、私は現状存在しているテストがどういうものなのかRustの#[test]なのかJSのtestなのか、何を検証しているものなのか知らないので、それをまとめてほしい。
次の担当モデルは、まず現在の差分とここに記した実行記録を確認し、追加の依頼に沿って作業すること。
本資料に残した未実施事項を、ユーザーが全て実装するよう依頼したと解釈しないこと。

ユーザーの依頼は次の順で進んだ。

1. 既存テストが Rust、JS、その他のどの形式で、何を検証しているかを整理する。
   最初は現状把握の依頼であり、この段階ではソースや設定を変更しなかった。
2. 「反映前検証は一般のメソッドにも適用されるのか、確認済み5メソッドだけなのか」「Kanon の緑表示と同じ判定なのか」を説明する。
3. 過去の49入力は5メソッド合計で小さいサイズに限ると確認したうえで、各合成解を様々な環境と引数で参照実装と比較する。
   範囲外の入力を含める必要はないとの指定がある。
4. `tests/paper_pipeline_tests.rs` と `insert_request()` が具体的に何を検査しているかを説明する。
5. 以上を別モデルへ渡せる引き継ぎ資料にする。これが直近の依頼。

作業上は、実ソースと新しい実行結果を根拠に説明する。
PBE の成功、JS への翻訳、メソッド全体の再実行、エディタへの反映、実際の UI での再実行を区別する。
テスト件数と検査入力数を混同しない。
日本語で説明し、命名、実装、テスト、コメントについてはプロジェクトの `AGENTS.md` に従う。
サブモジュールと既存の無関係な変更は触らない。

現在の作業場所と固定リビジョンは次のとおり。

| 対象 | 状態 |
| --- | --- |
| cwd | `/Users/taku/workspace/kenkyu/proj-ref/refsyn` |
| ブランチ | `main` |
| RefSyn HEAD | `86872f9f868dd30e8e60b1d2d690fa63b696170a` |
| Kanon | `60fa5c8e80d05a28dd12438ee1f4876a49bba79f` |
| external/escher-ts | `4e7c4bca45d98a0b5f2f76fd70486ed823c731f1` |

引き継ぎ資料の作成直前に `git status`、HEAD、サブモジュール、差分、主要ログを再確認した。
以下は未コミットの作業内容であり、コミットや push はしていない。

| パス | 変更内容 |
| --- | --- |
| `tests/whole_method_validation_tests.rs` | 既存の5テストから比較用 JS を呼ぶ共通関数を追加。既存の少数入力での検査も残した |
| `tests/js/list-method-differential.cjs` | 新規ファイル。現行の生成メソッドと参照実装を同じ入力で実行して状態を比較 |
| `docs/TESTS.md` | 比較条件、入力数、比較対象、保証の範囲、実行コマンドを追記 |
| `docs/TESTING_HANDOFF_2026-10-16.md` | この引き継ぎ資料 |

`.claude/` は作業開始時から未追跡だった。本作業では変更していない。
新規の `tests/js/` は未追跡なので、通常の `git diff --stat` だけでは内容が表示されない。
この資料も含め、次の担当者は実ファイルと `git status` を確認すること。

既存テストの調査では、現在使われる処理経路を次のように整理した。

| 層 | 形式と実行方法 | 調査時に再実行した結果 | 主な内容 |
| --- | --- | --- | --- |
| Rust | `#[test]` と `#[tokio::test]`、`cargo test --locked` | 55成功 | 実演読込み、操作グラフ、対応、テンプレート、PBE 問題、実合成と採用 |
| runtime | Node.js 標準の `node:test`、`node --test runtime/*.test.mjs` | 51成功 | JS 翻訳の意味一致とメソッド全体検証 |
| web | TypeScript と Vitest | 28成功、5ファイル | WASM 合成、hole 出力例の一致、helper の入力非変更、採用と Worker 制御 |
| Kanon | JavaScript と Jest | 重複モジュールを除外すると259成功、7失敗、4スキップ | 計装、記録、状態反映、エディタ反映、表示グラフ比較 |
| escher-ts | TypeScript と Vitest | 124成功、27ファイル | 型、値、式、コンポーネント、探索、ベンチマーク |
| Kanon の insert fixture | `npm --prefix Kanon run verify-refsyn-browser` | 全3実演の検証成功 | WASM → 合成 → 翻訳 → 全体検証 |

Rust の55件は、`src/escher_bridge.rs` 内の単体テスト4件と、`tests/` の結合テスト51件。
ファイル別では `paper_pipeline_tests.rs` 26件、`whole_method_validation_tests.rs` 10件、`fields_and_names_tests.rs` 7件、`boolean_values_tests.rs` 8件。
一部の Rust テストは、本物の escher-ts と Node.js を起動して生成 JS を実行する。
全モジュールに個別の単体テストが存在する構成ではない。
`escher_bridge.rs` の直接の4件はメモリ設定の解析を検査するもので、合成の意味を直接検査する4件ではない。
`main.rs` に専用テストはなく、HTTP 関連テストも主に `handle_synthesis` を直接呼び、実ポートの通信、route、CORS 全体を通すものではない。
`models.rs` は JSON 読込みを通して使われるが、専用の網羅的な往復テストは見当たらない。
`error.rs` の `RefSynError` には専用テストも利用箇所も見当たらなかった。

Kanon の実行には、二つの問題を分けて記録した。

- 通常の `npm test -- --runInBand` は、既存の `dist/external/escodegen/package.json` と `external/escodegen/package.json` の重複を検出し、主なテストファイルの読込みに失敗した。
- ファイルを削除せず、`--modulePathIgnorePatterns '<rootDir>/dist/'` を指定するとテスト本体を実行できた。
  この状態では259成功、7失敗、4スキップ。
- 7失敗は `testize-argument-overlay.test.js` に集中し、準備処理で呼ぶ `inferMethodParamNamesForCall` が未定義。
  Kanon 自身の CI は、このファイルを明示的に除外している。
- 4スキップは instrumentation 内の目視表示用2件と旧 AST 形状に依存する2件。

web の Vitest は `environment: "node"` で動く。
WASM と実合成器を使うテストがある一方、Worker を FakeWorker や直接関数呼出しで代替するテストもある。
Kanon の Jest では、エディタや合成応答を代替している箇所がある。
これらの成功を、実際のブラウザ操作全体を自動再現した証拠と説明しないこと。

ユーザーへの説明で訂正した要点は、Kanon の緑表示と反映前検証の区別である。

| 判定 | 実装と意味 |
| --- | --- |
| Kanon の緑表示 | `testize.js` の `checkActualGraph`／`testAndOverride` が表示グラフを比較し、`storedTest[...].passed` を更新。`updateMarker` が `testPassed` を使って呼出し位置を緑にする |
| 合成結果の反映前検証 | 別の `runtime/whole-method-validation.mjs` が型付き実演前状態を再構成し、生成コードを実行して期待状態と比較する |

反映前検証には、5メソッドだけを許すホワイトリストはない。
必要な型付き状態と constructor 定義を記録でき、対応可能な操作で実演が書かれていれば、ほかのメソッドにも適用される。
ただし、任意の JavaScript や外部状態まで検証できるわけではない。
保証対象は、取得した全実演と監視対象の範囲である。

検証器は同じ初期状態を二つ作り、一方に記録操作を適用して期待状態を作り、他方で生成メソッドを実行する。
値、フィールドの有無と属性、参照の同一性、共有参照、循環、切断された既存オブジェクト、生成数と生成オブジェクトの構造、戻り値、各 helper の実際の出力を比較する。
globalThis、指定の組込み prototype、対象クラスの静的プロパティと prototype に残る変更も監視する。
全ての副作用や途中で元に戻された変更を網羅するものではない。
結果は `passed`、`failed`、`unavailable` に分かれる。
`passed` 以外では採用対象の `code` と `composed_method_code` を消す。
Kanon はさらに全実演数の一致と合成中のソース未変更を確認してから反映する。
失敗時の通知はソース上では `console.warn`。専用の画面上の失敗表示まで検査したとは言っていない。

参照箇所は `runtime/whole-method-validation.mjs`、`src/validation.rs`、`src/lib.rs` の `handle_synthesis`、`web/src/lib/browser-runner.ts`、`Kanon/src/js/testize.js` の1332行付近、1743行付近、2077行付近、3109行付近。
行番号は編集で変わるため、関数名でも検索すること。

過去の49入力検査は、今回拡充したテストと区別する。

| 過去の検査 | 内容 |
| --- | --- |
| スクリプト | `docs/evaluation_cases/generated_code_validation/validate.mjs` |
| 読み込むコード | `reruns/2026-09-09-null-guard-removal` の保存済み応答 |
| スクリプトに記された生成元 commit | `3b323c5ac64852b91c2f3baaaabdb4cd4733daaa` |
| 入力数 | setAt 15、insert 15、append 5、prepend 5、popBack 5、setAt と insert の追加条件各2で計49 |
| サイズ | setAt／insert／append／prepend は長さ1〜5、popBack は長さ2〜6 |
| 比較対象 | 到達可能な値列と非循環性 |
| 調査時の再実行 | 49入力一致。ただし現行合成器で再合成した結果ではない |

この過去スクリプトは、不一致件数を JSON に出すが、不一致を終了コードへ変える処理は末尾にない。
この成功を現行 HEAD の一般化性能の証拠にしないこと。
`docs/evaluation_cases/reruns/` にある rotateRight の再実行や baseline の比較も、保存コードや個別実験の検査であり、標準の回帰テストに全て登録されているわけではない。

今回の比較テスト拡充は、以下の仕様で実装した。

1. 各 Rust テストが既存の実演 payload から現行の処理で合成し、全体検証を通過したメソッドと helper を得る。
2. `compare_with_reference` が、生成コードと、そのケースの `target_program.js` から抜き出した `Obj` クラスを JS 比較器へ JSON で渡す。
3. 比較器は参照側と生成側に別のクラスと初期状態を作る。
   生成側からは元の参照メソッドを削除し、生成コードだけを設定する。
   対象メソッドが生成コードに存在しない場合も失敗させる。参照実装への無意識なフォールバックを防ぐためである。
4. 同一の有効入力で両方を実行し、状態と戻り値を比較する。
   一つでも不一致や例外があれば Node の非ゼロ終了を介して Rust テストが失敗する。

合成は各メソッドにつきテスト実行中に1回行い、その合成解を多数の入力で実行する。
11,454回の合成を行ったわけではなく、学習用実演自体も多数の種類に変えたわけではない。
元の少数入力のテストも残し、setAt、insert、prepend の連続呼出しなどを削除していない。

| メソッド | 比較入力数 | 今回の最終実行 |
| --- | ---: | --- |
| setAt | 5,160 | 全件一致 |
| insert | 5,160 | 全件一致 |
| append | 531 | 全件一致 |
| prepend | 531 | 全件一致 |
| popBack | 72 | 全件一致 |
| 合計 | 11,454 | 全件一致 |

長さは1、2、3、5、8、16、32であり、1〜32の全ての長さを試したわけではない。
popBack は長さ2以上だけを試した。
値列は、異なる正整数、全部0、重複する正負の整数、固定 seed の疑似乱数の4種類。
疑似乱数は長さごとに `0x5eed + length` で初期化し、整数は -1000〜1000。
代入／挿入値は -10001、-1、0、1、10007、リストの両端の値を重複除去した集合から選ぶ。
setAt と insert は各長さの全有効添字を試す。
insert は i 番目の直後への挿入であり、末尾への挿入も含む。

3種類の環境は、先頭 receiver、共有された後続部分と切断オブジェクトがある場合、より大きいリストの途中が receiver の場合。
共有環境では、ノードの生成順序も逆転させている。
各状態には先頭、receiver、中間、末尾、receiver と同じ参照への別名を持たせる。
比較では、全既存オブジェクトに固定 ID を付け、追加の constructor 生成も追跡する。
切断されたノードや到達不能な生成も含め、全 own field、フィールドの有無と属性、クラス、参照と共有関係、戻り値を比較する。
比較器の同期実行には Node の VM で10秒の上限を設けている。

この新規比較器は、5つの `Obj`／`f`／`g` による整数リストメソッド向けであり、任意のクラスやフィールド名に使える汎用環境ではない。
constructor と初期状態作成のコードは両側で共用し、メソッドのアルゴリズムを独立した参照として比較する。
比較器は呼出し前後の状態を比べるもので、途中の全書込みや ambient な globals／prototypes を監視しない。
空 receiver、範囲外添字、singleton の popBack、任意の整数、任意の長さへの一般的正しさを保証しない。
ランダム操作列や正規化の意味保存を調べる体系的な property-based test を追加したわけでもない。

比較器自体の確認として、誤った値、添字のずれ、値は同じでも参照を複製、誤った戻り値、切断ノードの破壊、到達不能な余分な生成、null 代入の代わりにプロパティ削除、対象メソッド欠落の8種類を一時スクリプトから与え、全て非ゼロ終了になることを確認した。
この8種類はリポジトリの永続的な回帰テストには追加していない。
実行結果は `/private/tmp/refsyn-varied-comparator-negatives.log` に残る。

`paper_pipeline_tests.rs` についてユーザーへ説明した内容は以下。
`insert_request()` は論文の3実演を手書きした共通 fixture であり、一般的な参照実装ではない。

```text
初期状態          A39 → B27 → C53
insert(0, 89) 後  A39 → D89 → B27 → C53
insert(2, 46) 後  A39 → D89 → B27 → E46 → C53
insert(1, 71) 後  A39 → D89 → F71 → B27 → E46 → C53
```

| 検査 | 具体的な assertion の意味 |
| --- | --- |
| fixture の整合 | `tests/data/paper_insert.json` と `insert_request()` が同じ JSON である |
| 読込み | val が undefined、extra が null、next が欠落という3状態を区別する |
| 書込み列 | C.next を A → B → A と更新し、順番どおり3書込みを保持する |
| 対応付け | 第1実演の A → 第2実演の B、B → C、生成 D → 生成 E という対応を期待する |
| テンプレート | 生成1文、hole計算3文、書込み3文の7文になる |
| hole の出力 | h0 は [89,46,71]、挿入位置 h1 は [A,B,D]、後続 h2 は [B,C,B] |
| PBE の入力 | A.next は書込み前の B のまま。生成直後の状態と先行 hole を含め、自分と後続 hole を含めない |
| 拒否 | 未知の参照、不整合な旧参照、未対応操作、異なるメソッド、非同型などを拒否する |
| 実合成 | 最後まで通す一部のテストでは本物の合成器と検証器を動かし、全3実演の通過を確認する |

このファイルの多くは中間表現や拒否条件の検査であり、生成した insert を多数の追加入力で動かす検査ではない。
`single()` などで実演前状態や操作を差し替えるため、全26件が元の insert だけをそのまま処理しているわけでもない。
元 fixture のクラスソースは `class Node {}` だが、全体実行検証を行う正例では constructor が val と next を初期化するソースへ差し替えている。
今回追加した比較テストの学習用データは、`insert_request()` ではなく `docs/evaluation_cases/<case>/mold_payload.json`。
古い記録の型付き状態を補う既存の `with_typed_pre_state` と `with_typed_literal_input` を通す点も、そのまま残している。

再現の主なコマンドは次のとおり。
必要な依存が未導入なら、既存の `docs/TESTS.md` に従って固定サブモジュールと escher-ts を準備する。
この会話では新しい依存を追加していない。

```sh
cd /Users/taku/workspace/kenkyu/proj-ref/refsyn
pnpm --dir external/escher-ts run build
cargo test --locked --test whole_method_validation_tests matches_the_reference_on_unseen_inputs -- --nocapture
cargo test --locked
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
git diff --check
```

最初の比較テストは5件であり、その中で合計11,454入力を比較する。
`--nocapture` では合成処理の大きい診断出力も出るので、ログを保存して `reference comparison:` と `test result:` を検索すると確認しやすい。

調査したその他の実行方法は以下。

```sh
node --test runtime/*.test.mjs
npm run test:rebuild --prefix web
npm --prefix Kanon run verify-refsyn-browser
pnpm --dir external/escher-ts run test:run
cd Kanon
npx jest --runInBand --modulePathIgnorePatterns '<rootDir>/dist/' --testPathIgnorePatterns external
```

最後の Jest は既知の overlay 7失敗も含めて現状を確認するコマンドであり、全件成功のコマンドではない。
調査時は Node v22.17.0 を使用した。
escher-ts の dist と WASM は調査段階で現行ソースから再ビルドした。
wasm-bindgen は PATH 上になかったため、ローカルにあった0.2.114を一時的に PATH に追加して使用した。
次のモデルが同じ一時ツールパスを使えるとは限らないので、必要なら `docs/TESTS.md` に従って版を確認すること。

`cargo run` は現状サーバーを起動するだけで、汎用の検証 CLI ではない。
`scripts/run_escher.js` は PBE タスクから合成と翻訳を行い、`scripts/validate_method.mjs` は request／response／taskNames の JSON を標準入力から受け取って再実行検査する。
後者は検査結果を JSON の status で返すので、プロセス成功だけを検証成功とみなさない。
`test_api.sh` は古い `/analyze` と旧 payload を使い、assertion もないため、現行 API の自動検査として扱えない。今回は実行も修正もしていない。
ローカルの `Escher-Scala/` は追跡対象外で、現行経路は escher-ts。ScalaTest 5ファイルは調査対象の現行件数に含めず、実行していない。

検証記録は次のとおり。
`/private/tmp` の資料は一時ファイルなので、消えていたら保存済みの成功を現在の証拠として使わず再実行すること。

| 記録 | パス |
| --- | --- |
| 拡充前の詳細調査メモ | `/private/tmp/refsyn-test-inventory-2026-10-16.md`。拡充前の状態を記述しており、新規比較についてはこの資料と TESTS.md を優先 |
| 拡充前の各層の実行 | `/private/tmp/refsyn-audit-rust.log`、`refsyn-audit-runtime.log`、`refsyn-audit-web.log`、`refsyn-audit-escher.log`、`refsyn-audit-browser-fixture.log` |
| Kanon の通常実行失敗 | `/private/tmp/refsyn-audit-kanon.log` |
| Kanon の重複モジュール除外後 | `/private/tmp/refsyn-audit-kanon-isolated.log` |
| 過去コード49入力の再検査 | `/private/tmp/refsyn-audit-historical-validation.json` |
| 拡充後の Rust 全55件 | `/private/tmp/refsyn-varied-cargo-test.log` |
| 最終の5比較テストと各入力数 | `/private/tmp/refsyn-varied-reference-final.log` |
| Clippy | `/private/tmp/refsyn-varied-clippy.log` |
| 比較器に与えた誤った候補8種 | `/private/tmp/refsyn-varied-comparator-negatives.log` |

検証順序も区別する。
拡充後に Rust 全55件と Clippy を通した後、生成側から参照メソッドを削除して欠落候補を拒否する保護を比較器に加えた。
その最終の比較器に対して、5比較テストと誤った候補8種を再実行して成功を確認した。
最終の保護追加後に Rust 全55件を一括では再実行していないが、変更した比較器を呼ぶ5件は再実行済みで、ほかの50件はこの保護追加の影響を受けない。
最後に `cargo fmt --check` と `git diff --check` も成功した。

RefSyn の親 CI は、固定サブモジュール、escher-ts のビルド、Rust の fmt／Clippy／テスト、runtime、WASM 再ビルド、web、Kanon の insert fixture、Kanon の全体検証と receiver 記録の2ファイルを実行する。
escher-ts の124テスト全体と Kanon の全テストは親 CI の項目に含まれない。
この会話では GitHub Actions の最新結果を確認せず、実ブラウザの操作再現もしていない。
比較テスト拡充後に JS runtime／web／Kanon の全テストを再実行したわけでもない。これらの実装は変更していない。

次の担当モデルが続きを求められた場合は、まずユーザーの依頼範囲を確認する。
現状で残る検討対象には、追加比較テストのレビュー、他のメソッドへの拡張、Kanon のテスト起動と未実装 overlay の整理、実 UI を通す自動再現、操作列生成や意味保存の性質の検査などがある。
ただし、これらは未実施事項の一覧であって、全てを実装する合意ではない。
テストの追加をそのまま一般的正しさの証明や合成成功率の評価に読み替えず、どの固定コードと入力で何が一致したかを示すこと。
