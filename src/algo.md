# RefSyn パターン抽出アルゴリズム

## 概要

RefSynは、プログラム実行の操作ログから共通パターンを抽出し、ホール（穴）を含むテンプレートコードを生成するシステムです。

## 全体フロー

1. **操作ログの変換** - KanonからのJSONログをIR（中間表現）に変換
2. **依存関係分析** - 操作間の依存関係を分析し、有向グラフを構築
3. **正規化** - トポロジカルソートにより操作順序を正規化
4. **パターンマッチング** - 複数の操作列をマッチングして共通パターンを抽出
5. **ホール抽出** - 差異をホール（穴）として識別
6. **ASTテンプレート生成** - ホールを含むテンプレートコードを生成

## 詳細アルゴリズム

### 1. 操作の中間表現（IR）

#### 操作の種類（OpKind）
```rust
pub enum OpKind {
    // ノード操作
    AddNode { id: NodeId, is_literal: bool, label: String },
    EditNode { id: NodeId, is_literal: bool, label: String },
    DeleteNode { id: NodeId },
    
    // エッジ操作
    AddEdge { from: NodeId, to: NodeId, label: String },
    EditEdgeReference { from: NodeId, old_to: NodeId, new_to: NodeId, label: String },
    EditEdgeLabel { from: NodeId, to: NodeId, old_label: String, new_label: String },
    DeleteEdge { from: NodeId, to: NodeId, label: String },
    
    // 変数操作
    AddVariable { to: NodeId, label: String },
    EditVariableReference { old_to: NodeId, new_to: NodeId, label: String },
    EditVariableLabel { to: OpId, old_label: String, new_label: String },
    DeleteVariable { to: OpId, label: String }
}
```

### 2. 依存関係分析

#### build_graph関数
```
入力: 操作列 ops: &[Op]
出力: (依存グラフ, ノードインデックスマップ)

for 各操作 u in ops:
    for 各後続操作 v in ops[u+1..]:
        if u と v に依存関係がある:
            グラフに辺を追加 (u → v, EdgeTag)
```

#### 依存関係の種類
- **GenUse**: 生成-使用の依存関係（AddNode → その他の操作）
- **Overwrite**: 上書きの依存関係（同じプロパティへの操作）

#### 依存関係判定ルール
1. `AddNode → 任意の操作`: 後続操作がそのNodeIDを参照する場合
2. `AddEdge → AddEdge`: 同じfromとlabelを持つ場合
3. `AddEdge → EditEdgeReference`: 同じfromとlabelを持つ場合
4. `EditEdgeReference → EditEdgeReference`: 同じfromとlabelを持つ場合
5. `AddVariable → AddVariable`: 同じlabelを持つ場合
6. `AddVariable → EditVariableReference`: 同じlabelを持つ場合
7. `EditVariableReference → EditVariableReference`: 同じlabelを持つ場合

### 3. 正規化（トポロジカルソート）

#### canonical_order関数
```
入力: 操作列 ops: &[Op]
出力: 正規化された操作ID順序

1. 依存グラフを構築
2. 各ノードの入次数を計算
3. 優先度付きキューを初期化（入次数0のノードを追加）
4. Kahnのアルゴリズム実行：
   while キューが空でない:
       最高優先度のノードを取り出し
       結果リストに追加
       隣接ノードの入次数を減らし
       入次数0になったノードをキューに追加
```

#### 操作優先度
```rust
fn get_label_priority(op: &Op) -> (i32, &str) {
    match &op.kind {
        OpKind::AddNode { .. } => (0, ""),        // 最高優先度
        OpKind::AddEdge { .. } => (1, ""),
        OpKind::EditEdgeReference { .. } => (1, ""),
        OpKind::AddVariable { .. } => (2, ""),
        OpKind::EditVariableReference { .. } => (2, ""),
        _ => (3, ""),                             // 最低優先度
    }
}
```

### 4. パターンマッチング

#### match_graphs関数
```
入力: 2つの操作列 a, b
出力: MatchResult { common_ops, holes }

1. 両方の操作列を正規化
2. 同じ位置の操作を比較:
   for i in 0..min(a.len(), b.len()):
       if 操作種類が同じ:
           詳細比較を実行
           差異をホールとして記録
           共通操作として追加
```

#### 操作比較ルール

**AddNode比較**:
- `is_literal`が一致する場合のみマッチ
- `label`が異なる場合はConstホールを作成

**AddEdge比較**:
- `label`が一致する場合のみマッチ
- `from`, `to`が構造的に異なる場合はRefホールを作成

**EditEdgeReference比較**:
- `label`が一致する場合のみマッチ
- `from`, `old_to`, `new_to`それぞれについて構造的差異をチェック

### 5. ホール抽出

#### ホールの種類
```rust
pub enum Hole {
    Const { placeholder: String, values: Vec<String> },  // 定数の差異
    Ref { placeholder: String, refs: Vec<String> },      // 参照の差異
}
```

#### 構造的同等性判定
```rust
fn is_structurally_equivalent(id_a, id_b, a_ops, b_ops, id_mapping) -> bool {
    // 1. 同じIDの場合は同等
    // 2. main-new（既存）と__temp（新規）は異なるものとして扱う
    // 3. IDマッピングで対応している場合は同等
    // 4. AddNode操作を検索し、is_literalとlabelが一致すれば構造的に同等
}
```

### 6. テンプレートAST生成

#### generate_template_ast関数
```
入力: MultiMatchResult, MemoEnv
出力: テンプレートProgram

1. ID正規化:
   - 決定論的順序でIDをソート
   - obj_0, obj_1, ... のように変数名を割り当て
   - "this"や"main-new"は特別扱い

2. ホール番号正規化:
   - Hole1, Hole2, ... のように連番を割り当て

3. AST文生成:
   for 各共通パターン操作:
       match 操作種類:
           AddNode → VarDecl文
           AddEdge → Assign文
           EditEdgeReference → Assign文
           AddVariable → VarDecl文
```

#### ID → AST式変換
```rust
fn get_normalized_expr_for_id_with_mapping_and_holes(id, id_mapping, holes, ...) -> Expr {
    // 1. ホールに含まれるかチェック
    // 2. 定数ホールの場合はHole式を返す
    // 3. 参照ホールの場合は通常の変数マッピングを使用
    // 4. "this"や"main-new"の特別処理
    // 5. デフォルト変数名の生成
}
```

### 7. メイン処理フロー

#### find_common_pattern_from_operations関数
```
入力: operations_list, memo_envs
出力: (Option<Program>, HashMap<String, Vec<String>>)

1. 各操作ログをIRに変換
2. 各操作列をトポロジカルソート
3. 複数操作列のマッチング:
   if 2つ以上の操作列:
       パターンマッチングを実行
       テンプレートASTを生成
       ホール情報を抽出
   else:
       単一操作列からASTを生成
```

## アルゴリズムの特徴

### 依存関係を考慮した正規化
- 操作間の依存関係を明示的にモデル化
- トポロジカルソートにより実行可能な順序を保証
- 優先度により決定論的な順序を実現

### 構造的マッチング
- 操作の種類だけでなく、構造的な同等性を判定
- IDの意味（既存オブジェクト vs 新規オブジェクト）を考慮
- 柔軟なホール抽出により汎用的なパターンを生成

### ホール番号の正規化
- 一貫したホール識別子（Hole1, Hole2, ...）
- 定数ホールと参照ホールの区別
- テンプレート内での適切なホール配置

このアルゴリズムにより、複数のプログラム実行トレースから共通パターンを抽出し、差異をホールとして抽象化したテンプレートコードを生成できます。
