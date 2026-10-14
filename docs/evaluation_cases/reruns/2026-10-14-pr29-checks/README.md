# PR #29の検査と負例の保存記録

対象は `edf6ac1bb9263f8a509cf4e9193522d5927a804f` である。
以下は2026-10-14に取得したログであり、文書整理時に負例を再実行した結果ではない。
適用前と復元後のHEADとgit statusは [00-state.txt](00-state.txt) に保存されている。
どちらも対象checkoutの差分なしと記録されている。

| ファイル | 記録された結果 |
| --- | --- |
| [01-tsc.txt](01-tsc.txt) | `npx tsc --noEmit -p .` は終了コード2。35件の診断があり、34件はescher-ts側の依存の型定義、1件は `src/worker.ts`。新しいテストファイルの診断は含まれないが、全体の型検査は成功していない |
| [02-baseline-vitest.txt](02-baseline-vitest.txt) | 対象テストファイルの14件が成功。web全体28件の実行ログではない |
| [03-negative-throwing-nextOf.txt](03-negative-throwing-nextOf.txt) | `nextOf` の翻訳を即座に例外を投げるhelperに差し替え、部品別検査1件が失敗。期待値 `ref 1` に対して `failure` を検出 |
| [04-negative-traversal-write.txt](04-negative-traversal-write.txt) | 走査中に `seen` を書き込んで削除する処理を注入し、copy_flagと部品別検査の2件が失敗 |

負例のファイルはmutationのdiffとvitestの出力を含む。
ログとdiffの末尾空白、末尾の空行は、取得した出力のまま保持している。
同じ負例を再現する場合は、依存とWASMを準備した専用checkoutで該当diffを適用し、次を実行する。
初期準備は [web検査の手順](../../../TESTS.md#web-tests-from-a-clean-checkout) を参照する。

```bash
cd web
npm test -- src/lib/binding-conditions.test.ts
```

ログ先頭の型検査の注記は「filtered」と書かれているが、その後には依存を含む診断が保存されている。
「エラーは2件」という以前の作業報告は、このログに一致しない。
依存の型解決の問題を含むため、PRの変更が全ての診断の原因だとも、この記録だけでは判断できない。

この検査が扱うのは、論文のinsertと真偽値のfixture、列挙した部品と入力である。
未観測入力での純粋性、全体の型検査成功、主要5ケースの新規評価は示さない。
