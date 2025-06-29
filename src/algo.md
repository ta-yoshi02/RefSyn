# 共通部分抽出アルゴリズム

RefSyn は複数のオブジェクト操作列から "最大部分被覆" を求め，差異をホールとして残したテンプレートプログラムを生成する。
ここでは実装の流れを自然言語と擬似コードを交えて説明する。

## アルゴリズム概要

1. **操作ログの変換** - KanonからのJSONログをIR（中間表現）に変換
2. **依存関係分析** - 操作間の依存関係を分析し、有向グラフを構築
3. **正規化** - トポロジカルソートにより操作順序を正規化
4. **パターンマッチング** - 複数の操作列をマッチングして共通パターンを抽出
   - **線形マッチング**: 順序に基づく直接比較
   - **構造的同型マッチング**: 依存関係グラフの同型性に基づく比較
   - **詳細構造マッチング**: 操作の内部構造を明示化した詳細グラフによる比較
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
            グラフに辺を追加 (u → v)
```
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

#### match_graphs関数（線形比較）
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

#### match_graphs_with_isomorphism関数（構造的同型比較）
```
入力: 2つの操作列 a, b
出力: Option<MatchResult { common_ops, holes }>

1. 両方の操作列を正規化
2. 依存グラフを構築:
   graph_a = build_graph(a)
   graph_b = build_graph(b)
   
3. ノード間のIDマッピングを作成:
   id_mapping = build_id_mapping_between_graphs(a, b)
   
4. グラフ同型判定:
   if is_isomorphic(graph_a, graph_b, id_mapping):
       同型と判定され、詳細比較を実行
       差異をホールとして記録
       return Some(MatchResult)
   else:
       return None  // 非同型のため比較不可
```

**構造的同型判定の詳細**:

##### build_id_mapping_between_graphs関数
```
入力: 操作列 a, b
出力: IDマッピング HashMap<NodeId, NodeId>

id_mapping = empty_map()

// 1. "this" と "main-new" の特別マッピング
id_mapping.insert("this", "this")
if both contain "main-new":
    id_mapping.insert("main-new", "main-new")

// 2. AddNode操作による双方向マッピング
for op_a in a:
    if op_a is AddNode:
        for op_b in b:
            if op_b is AddNode and 
               op_a.is_literal == op_b.is_literal and
               op_a.label == op_b.label:
                // 可能な組み合わせをマッピング候補に追加
                candidate_mappings.push((op_a.id, op_b.id))

// 3. 最適なマッピングを選択（貪欲法）
// 構造的同等性を最大化するマッピングを選択
```

##### is_isomorphic関数
```
入力: graph_a, graph_b, id_mapping
出力: bool

// 1. ノード数チェック
if graph_a.node_count() != graph_b.node_count():
    return false

// 2. エッジ数チェック  
if graph_a.edge_count() != graph_b.edge_count():
    return false

// 3. ノード互換性チェック
for node_a in graph_a.nodes():
    if let Some(node_b) = id_mapping.get(node_a):
        if !are_nodes_compatible(node_a, node_b, a_ops, b_ops):
            return false

// 4. エッジ互換性チェック
for edge_a in graph_a.edges():
    mapped_from = id_mapping.get(edge_a.from)
    mapped_to = id_mapping.get(edge_a.to)
    
    if !graph_b.has_edge(mapped_from, mapped_to):
        return false
        
    if !are_edges_compatible(edge_a, corresponding_edge_b):
        return false

return true
```

##### are_nodes_compatible関数（ノード互換性判定）
```
入力: node_a, node_b, 操作列 a, b
出力: bool

op_a = find_add_node_operation(node_a, a)
op_b = find_add_node_operation(node_b, b)

// 1. リテラル性の一致
if op_a.is_literal != op_b.is_literal:
    return false

// 2. ラベル一致判定（柔軟性有り）
// 現在の実装では厳密一致を要求
if op_a.label != op_b.label:
    return false

// 【提案】構造的マッチングの場合：
// - 操作タイプ（メソッド呼び出し、プロパティアクセス等）で判定
// - ラベルの完全一致は不要（差異はホールとして抽出）

return true
```

**同型検出の利点**:
- **構造的同等性**: 操作の順序に依存しない比較
- **柔軟なマッチング**: 依存関係が保持されていれば異なる順序でもマッチ
- **高精度**: 構造的に異なるパターンの誤マッチを防止

**計算量**:
- グラフ構築: O(n²) (n = 操作数)
- 同型判定: O(V + E) (V = ノード数, E = エッジ数)
- 全体的に線形比較より高コストだが、精度向上と引き換え

#### match_graphs_with_flexible_structure関数（詳細構造マッチング）
```
入力: 2つの操作列 a, b
出力: bool

1. 詳細構造グラフを構築:
   graph_a = build_detailed_structure_graph(a)
   graph_b = build_detailed_structure_graph(b)
   
2. 柔軟な同型判定:
   is_isomorphic_matching(graph_a, graph_b, node_matcher, edge_matcher)
```

**詳細構造グラフの特徴**:

##### build_detailed_structure_graph関数
```
入力: 操作列 ops: &[Op]
出力: (詳細グラフ, ノードインデックスマップ)

// 1. 各操作をOperationノードとして追加
for op in ops:
    graph.add_node(DetailedNode::Operation(op.id))

// 2. 操作の詳細要素をノードとして追加し、エッジで接続
for op in ops:
    match op.kind:
        AddNode { id, is_literal, label }:
            // ノードIDを追加
            node_id_node = DetailedNode::NodeId(id)
            graph.add_edge(op_node, node_id_node, DetailedEdge::ReferencesNodeId)
            
            // ノードタイプを追加
            type_node = DetailedNode::NodeType(is_literal)
            graph.add_edge(op_node, type_node, DetailedEdge::SpecifiesNodeType)
            
            // ラベル/リテラル値を追加
            if is_literal:
                literal_node = DetailedNode::Literal(label)
                graph.add_edge(op_node, literal_node, DetailedEdge::UsesLiteral)
            else:
                label_node = DetailedNode::Label(label)
                graph.add_edge(op_node, label_node, DetailedEdge::UsesLabel)
                
        AddEdge { from, to, label }:
            // from/toノードIDを方向性エッジで接続
            from_node = DetailedNode::NodeId(from)
            graph.add_edge(op_node, from_node, DetailedEdge::EdgeDirection("from"))
            
            to_node = DetailedNode::NodeId(to)
            graph.add_edge(op_node, to_node, DetailedEdge::EdgeDirection("to"))
            
            label_node = DetailedNode::Label(label)
            graph.add_edge(op_node, label_node, DetailedEdge::UsesLabel)
```

##### 詳細ノード種別
```rust
pub enum DetailedNode {
    Operation(OpId),           // 操作自体
    NodeId(String),           // ノードID（__temp1, main-new1等）
    Literal(String),          // リテラル値（"0", "hello"等）
    Label(String),            // ラベル（"val", "next"等）
    NodeType(bool),          // ノードタイプ（true=literal, false=object）
}
```

##### 詳細エッジ種別
```rust
pub enum DetailedEdge {
    ReferencesNodeId,         // 操作がノードIDを参照
    UsesLabel,               // 操作がラベルを使用
    UsesLiteral,            // 操作がリテラル値を使用
    SpecifiesNodeType,      // 操作がノードタイプを指定
    EdgeDirection(String),   // エッジの方向性（"from", "to", "old_to", "new_to"）
}
```

##### 柔軟な同型マッチング
```rust
node_matcher = |node_a, node_b| {
    match (node_a, node_b) {
        // 操作ノード同士は常にマッチ（種類は接続ノードで判定）
        (DetailedNode::Operation(_), DetailedNode::Operation(_)) => true,
        
        // ノードIDは異なっていても良い（ホールとして扱う）
        (DetailedNode::NodeId(_), DetailedNode::NodeId(_)) => true,
        
        // リテラル値は異なっていても良い（ホールとして扱う）
        (DetailedNode::Literal(_), DetailedNode::Literal(_)) => true,
        
        // ラベルとノードタイプは一致が必要（構造の重要な部分）
        (DetailedNode::Label(a), DetailedNode::Label(b)) => a == b,
        (DetailedNode::NodeType(a), DetailedNode::NodeType(b)) => a == b,
        
        _ => false  // 異なる種類のノードはマッチしない
    }
};

edge_matcher = |edge_a, edge_b| {
    edge_a == edge_b  // エッジ種類は完全一致が必要
};
```

**詳細構造マッチングの利点**:
- **細粒度分析**: 操作の内部構造（使用するID、リテラル値、ラベル）を明示的に比較
- **ホール対応**: 具体的な値（ノードID、リテラル値）の差異を許容しつつ構造的類似性を判定
- **関係性保持**: 同じノードIDが複数の操作で使用される関係性を正確に捉える
- **精密マッチング**: 従来の依存関係グラフでは見落とされる細かな構造的差異を検出

**使用例**:
```javascript
// 例1: append操作
ops_a = [
    AddNode("__temp1", false, "Node"),
    AddNode("__temp2", true, "0"),        // リテラル値 "0"
    AddEdge("__temp1", "__temp2", "val"),
    AddEdge("main-new1", "__temp1", "next")
]

ops_b = [
    AddNode("__temp3", true, "3"),        // リテラル値 "3"（異なる）
    AddNode("__temp4", false, "Node"),
    AddEdge("__temp1", "__temp4", "next"),
    AddEdge("__temp4", "__temp3", "val")
]

// 詳細構造マッチング: true
// - 同じ操作パターン（AddNode×2, AddEdge×2）
// - 同じラベル使用（"Node", "val", "next"）
// - 同じノードタイプ（literal, object）
// - リテラル値（"0" vs "3"）とノードID差異は許容
```

**計算量**:
- 詳細グラフ構築: O(n) (n = 操作数)
- 同型判定: O(V + E) (V = 詳細ノード数, E = 詳細エッジ数)
- メモリ使用量: 従来の依存グラフより多い（操作の詳細要素を明示的にノード化）

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

## アルゴリズムの改良提案

### 3段階のマッチング戦略

現在の実装では複数のマッチング手法を提供しており、研究目的に応じて最適な戦略を選択できます：

#### 1. 線形マッチング（Linear Matching）
```rust
// match_graphs関数での順序ベース比較
fn linear_matching(a: &[Op], b: &[Op]) -> MatchResult {
    // 正規化後の順序で直接比較
    for i in 0..min(a.len(), b.len()) {
        if operations_compatible(a[i], b[i]) {
            // 共通操作として記録
        }
    }
}
```
**利点**: 計算コストが最も低い、実装が簡単
**欠点**: 順序の違いに敏感、柔軟性に欠ける

#### 2. 構造的同型マッチング（Structural Isomorphism）
```rust
// match_graphs_with_isomorphism関数での依存関係グラフ比較
fn structural_isomorphism(a: &[Op], b: &[Op]) -> bool {
    let (graph_a, _) = build_graph(a);
    let (graph_b, _) = build_graph(b);
    
    is_isomorphic_matching(graph_a, graph_b, node_matcher, edge_matcher)
}
```
**利点**: 操作順序に依存しない、依存関係を考慮
**欠点**: 操作の内部構造（使用するID等）の詳細は考慮しない

#### 3. 詳細構造マッチング（Detailed Structure Matching）【新実装】
```rust
// match_graphs_with_flexible_structure関数での詳細構造グラフ比較
fn detailed_structure_matching(a: &[Op], b: &[Op]) -> bool {
    let (graph_a, _) = build_detailed_structure_graph(a);
    let (graph_b, _) = build_detailed_structure_graph(b);
    
    is_isomorphic_matching(graph_a, graph_b, flexible_node_matcher, edge_matcher)
}
```
**利点**: 操作の内部構造を明示化、ホール抽出に最適、関係性を正確に捉える
**欠点**: 計算コストとメモリ使用量が最も高い

### マッチング手法の比較

| 手法 | 計算量 | メモリ | 順序依存 | 内部構造 | ホール精度 | 適用場面 |
|------|--------|--------|----------|----------|------------|----------|
| 線形 | O(n) | 最小 | 有り | 無し | 低 | 高速プロトタイプ |
| 構造的同型 | O(n²) | 中 | 無し | 部分的 | 中 | 一般的な用途 |
| 詳細構造 | O(n) | 高 | 無し | 完全 | 高 | 精密解析 |

### 柔軟性の段階

#### 1. 厳密マッチング（Exact Matching）
```rust
fn strict_node_matcher(node_a, node_b) -> bool {
    node_a == node_b  // 完全一致
}
```

#### 2. 意味的マッチング（Semantic Matching）
```rust
fn semantic_node_matcher(node_a, node_b) -> bool {
    match (node_a, node_b) {
        (DetailedNode::Label(a), DetailedNode::Label(b)) => a == b,  // ラベルは一致
        (DetailedNode::NodeType(a), DetailedNode::NodeType(b)) => a == b,  // タイプは一致
        (DetailedNode::NodeId(_), DetailedNode::NodeId(_)) => true,  // IDは柔軟
        (DetailedNode::Literal(_), DetailedNode::Literal(_)) => true,  // 値は柔軟
        _ => false
    }
}
```

#### 3. 構造的マッチング（Structural Matching）【現在の詳細構造実装】
```rust
fn flexible_node_matcher(node_a, node_b) -> bool {
    // 操作種類のみで判定、具体的な値は全てホールとして扱う
    std::mem::discriminant(node_a) == std::mem::discriminant(node_b)
}
```

### パフォーマンス最適化

#### グラフ同型判定の高速化
1. **事前フィルタリング**: ノード数・エッジ数の事前チェック
2. **次数分布比較**: 各ノードの入次数・出次数分布の一致確認
3. **ラベル頻度分析**: ラベルの出現頻度による早期枝刈り

```rust
fn quick_isomorphism_check(graph_a, graph_b) -> bool {
    // 1. 基本統計チェック
    if graph_a.node_count() != graph_b.node_count() ||
       graph_a.edge_count() != graph_b.edge_count() {
        return false;
    }
    
    // 2. 次数分布チェック
    let degree_dist_a = compute_degree_distribution(graph_a);
    let degree_dist_b = compute_degree_distribution(graph_b);
    if degree_dist_a != degree_dist_b {
        return false;
    }
    
    // 3. ラベル頻度チェック
    let label_freq_a = compute_label_frequencies(graph_a);
    let label_freq_b = compute_label_frequencies(graph_b);
    if label_freq_a != label_freq_b {
        return false;
    }
    
    true  // 詳細チェックへ進む
}
```

### 適用場面の使い分け

- **高速プロトタイピング**: 線形マッチングで基本的なパターン発見
- **一般的な解析**: 構造的同型マッチングで順序に依存しないパターン抽出
- **精密なホール抽出**: 詳細構造マッチングで正確な差異分析
- **リファクタリング支援**: 意味的マッチングで関連する操作グループ抽出
- **厳密検証**: 厳密マッチングで完全一致パターンの特定

この多段階アプローチにより、研究目的や解析対象に応じて最適なマッチング戦略を選択でき、計算コストと精度のトレードオフを調整できます。

### 実装における優先順位

RefSynでは以下の優先順位でマッチング手法を適用します：

1. **詳細構造マッチング**: 最も精密な解析が可能
2. **構造的同型マッチング**: バランスの取れた手法
3. **線形マッチング**: フォールバック手法

これにより、可能な限り高精度な結果を提供しつつ、計算不可能な場合にも対応できる柔軟なシステムを実現しています。
