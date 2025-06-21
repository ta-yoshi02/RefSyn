# 共通部分抽出アルゴリズム

RefSyn は複数のオブジェクト操作列から "最大部分被覆" を求め，差異をホールとして残したテンプレートプログラムを生成する。
ここでは実装の流れを自然言語と擬似コードを交えて説明する。

## アルゴリズム概要

1. **操作列の正規化**
   - 各操作列を IR に変換し，依存グラフを構築する。
   - 依存グラフをトポロジカルソートし，決定論的な順序で並べ替える（`canonical_order`）。
2. **共通部分の探索**
   - 2 つの正規化済み操作列を `match_graphs` で比較する。
   - 操作の種類と構造的同等性を基に一致を判定し，異なる部分はホールとして記録する。
3. **テンプレート生成**
   - 得られた共通操作列とホール情報から `generate_template_ast` を実行し AST を構築する。
   - ID やホール番号を正規化し，最終的なテンプレートプログラムを得る。

以下に処理全体を表す擬似コードを示す。各ステップの後に要点を日本語で補足する。

```pseudo
procedure extract_template(logs: List<OpLog>, envs: List<MemoEnv>) -> (Program, HoleMap)
    ops_lists := []
    for log in logs do
        ir := convert_operations_to_ir(log)
        ordered := sort_by(canonical_order(ir))
        ops_lists.append(ordered)
    end for

    // ここでは 2 列のみを対象とする
    result := match_graphs(ops_lists[0], ops_lists[1])

    tmpl := generate_template_ast(result, envs[0])
    holes := extract_holes_from_match_result(result)
    return (tmpl, holes)
end procedure
```

上記の `extract_template` は操作ログを受け取り、まず `canonical_order` に従って各ログを決定論的な並びに変換する。ここでは二つのログを比較しているが、拡張により複数列にも対応可能である。

## 主要処理

### 依存グラフの構築
`build_graph` は操作間の生成–使用関係や上書き関係を調べ，辺に `GenUse` または `Overwrite` を付与して有向グラフを作る．このグラフを `canonical_order` が利用してトポロジカル順序を決定する。

生成–使用関係は「あるオブジェクトを生成してから利用する」流れを保証し，上書き関係は同じプロパティへの連続操作を整列させる。グラフをトポロジカルソートすることで、ユーザー操作に依存しない一貫した順序を得られる。

### 操作のマッチング
`match_graphs` は 2 つの正規化済み操作列を走査して共通部分を求める。各操作ペアを次のように比較する。

- `AddNode` では `is_literal` が一致し，ラベルが異なる場合に定数ホールを生成する。
- `AddEdge` では `label` が等しい場合のみ一致とし，参照先が異なれば参照ホールを生成する。
- `EditEdgeReference` では `from` と `label` が等しいことを確認し，`new_to` が構造的に等価でなければホールとする。

マッチの過程で生成したホールの一覧が `MatchResult.holes` に格納される。

擬似コードで表すと以下のようになる。

```pseudo
for (op_a, op_b) in zip(ops_a, ops_b):
    if kind(op_a) != kind(op_b):
        break
    if kind is AddNode and op_a.is_literal == op_b.is_literal:
        if op_a.label != op_b.label:
            create_const_hole(op_a.label, op_b.label)
    if kind is AddEdge and op_a.label == op_b.label:
        check_reference(op_a.from, op_b.from)
        check_reference(op_a.to, op_b.to)
    ...
```

ここで `check_reference` は ID が構造的に等しいかを検査し，異なる場合に参照ホールを登録する補助関数である。

### テンプレートの構築
`generate_template_ast` は `MatchResult` の共通操作を AST に変換する。ID を `obj_0`, `obj_1`, ... のように割り当て，ホールは `Hole1`, `Hole2` と連番で置き換える。これにより同じ操作列を入力すれば常に同一のテンプレートが得られる。

この段階でホール情報を `HoleMap` としてまとめる。テンプレートではホールが占める位置を保持し，外部合成器に与える際にこの表を利用して変数や定数を補完する。

---
このアルゴリズムにより，複数の操作例から共通パターンを抽出し，差分をホールとして表すテンプレートプログラムが得られる。RefSyn はこのテンプレートを既存合成器に渡し，部分問題を解かせることで最終的なプログラム合成を行う。
