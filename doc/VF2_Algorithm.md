# VF2アルゴリズム: グラフ同型判定アルゴリズムの詳細解説

## 概要

VF2（Vento-Foggia 2）アルゴリズムは、2つのグラフが同型（isomorphic）であるかを判定する効率的なアルゴリズムです。Rustの`petgraph`ライブラリの`is_isomorphic_matching`関数もこのVF2アルゴリズムを基盤としています。

## VF2アルゴリズムの基本原理

### 1. グラフ同型判定問題

**グラフ同型判定問題**とは、2つのグラフG1 = (V1, E1)とG2 = (V2, E2)について、頂点の対応関係（写像）f: V1 → V2が存在し、以下の条件を満たすかを判定する問題です：

- |V1| = |V2| かつ |E1| = |E2|
- すべての辺 (u, v) ∈ E1 について、(f(u), f(v)) ∈ E2
- すべての辺 (x, y) ∈ E2 について、対応する (f⁻¹(x), f⁻¹(y)) ∈ E1

### 2. VF2アルゴリズムの戦略

VF2アルゴリズムは**バックトラッキング探索**を基盤とした状態空間探索アルゴリズムです：

```
状態 = 部分的な頂点対応関係（マッピング）
目標 = 完全な同型写像の発見
制約 = 構造的整合性（syntactic feasibility）
     + 意味的整合性（semantic feasibility）
```

## アルゴリズムの詳細

### 1. 状態表現

VF2では、探索状態を以下の集合で表現します：

- **M(s)**: 現在の部分マッピング（確定した頂点対応）
- **T1(s), T2(s)**: 候補頂点集合（マッピング済み頂点に隣接する未マッピング頂点）
- **T1_out(s), T2_out(s)**: 外部候補集合（T1, T2以外の候補頂点）

### 2. 探索手順

```python
def vf2_algorithm(G1, G2):
    # 初期状態
    M = {}  # 空のマッピング
    T1, T2 = set(), set()  # 候補頂点集合
    
    # 再帰的探索
    return match(M, T1, T2)

def match(M, T1, T2):
    # 終了条件: 全頂点がマッピング済み
    if len(M) == len(G1.vertices):
        return M  # 同型写像発見
    
    # 候補ペア生成
    for (u, v) in candidate_pairs(T1, T2):
        # 実行可能性チェック
        if is_feasible(u, v, M):
            # マッピング拡張
            M_new = M ∪ {(u, v)}
            T1_new, T2_new = update_candidates(T1, T2, u, v)
            
            # 再帰探索
            result = match(M_new, T1_new, T2_new)
            if result is not None:
                return result
            
            # バックトラック
            # （次の候補ペアを試行）
    
    return None  # 同型写像なし
```

### 3. 実行可能性判定（Feasibility Rules）

VF2の効率性の鍵は、早期枝刈りのための実行可能性ルールです：

#### 構造的実行可能性（Syntactic Feasibility）

```python
def syntactic_feasibility(u, v, M):
    # R_pred: 前方隣接関係の整合性
    for (u_i, v_i) in M:
        # u_i → u の辺があるなら、v_i → v の辺も必要
        if (u_i, u) in E1 and (v_i, v) not in E2:
            return False
        if (v_i, v) in E2 and (u_i, u) not in E1:
            return False
    
    # R_succ: 後方隣接関係の整合性
    for (u_j, v_j) in M:
        # u → u_j の辺があるなら、v → v_j の辺も必要
        if (u, u_j) in E1 and (v, v_j) not in E2:
            return False
        if (v, v_j) in E2 and (u, u_j) not in E1:
            return False
    
    # R_in/R_out: 候補集合サイズの整合性
    return check_candidate_set_consistency(u, v)
```

#### 意味的実行可能性（Semantic Feasibility）

```python
def semantic_feasibility(u, v):
    # 頂点ラベル/属性の一致判定
    if node_match is not None:
        return node_match(G1.node[u], G2.node[v])
    return True
```

### 4. 候補ペア生成順序

VF2の性能は候補ペア選択順序に大きく依存します：

```python
def candidate_pairs(T1, T2):
    # 優先順位:
    # 1. T1 × T2 (隣接候補ペア)
    # 2. T1_out × T2_out (外部候補ペア)
    
    if T1 and T2:
        # 隣接候補から選択（制約が多く早期枝刈り可能）
        return [(u, v) for u in T1 for v in T2]
    else:
        # 外部候補から選択
        return generate_remaining_pairs()
```

## RefSynプロジェクトでの利用

### 1. `petgraph::algo::is_isomorphic_matching`

```rust
use petgraph::algo::is_isomorphic_matching;

// 基本的な同型判定
let is_iso = is_isomorphic_matching(
    &graph_a, 
    &graph_b,
    |node_a, node_b| node_a == node_b,  // ノード比較関数
    |edge_a, edge_b| edge_a == edge_b   // エッジ比較関数
);
```

### 2. 構造的同型マッチング（ir.rs）

```rust
pub fn match_graphs_with_isomorphism(a: &[Op], b: &[Op]) -> bool {
    use petgraph::algo::is_isomorphic_matching;
    
    let (graph_a, _) = build_graph(a);
    let (graph_b, _) = build_graph(b);
    
    is_isomorphic_matching(
        &graph_a,
        &graph_b,
        |_, _| true,  // ノード（操作ID）は任意マッチ
        |_, _| true   // エッジ（依存関係）は任意マッチ
    )
}
```

### 3. 詳細構造マッチング（build_graph_detailed.rs）

```rust
pub fn match_graphs_with_flexible_structure(a: &[Op], b: &[Op]) -> bool {
    let (graph_a, _) = build_detailed_structure_graph(a);
    let (graph_b, _) = build_detailed_structure_graph(b);
    
    is_isomorphic_matching(
        &graph_a,
        &graph_b,
        |node_a, node_b| {
            match (node_a, node_b) {
                // 操作ノード: 種類が一致すればOK
                (DetailedNode::Operation(_), DetailedNode::Operation(_)) => true,
                // ノードID: 具体値が異なってもOK（構造的位置が重要）
                (DetailedNode::NodeId(_), DetailedNode::NodeId(_)) => true,
                // ラベル: 同じラベルなら一致
                (DetailedNode::Label(l1), DetailedNode::Label(l2)) => l1 == l2,
                // リテラル: 異なる値でもOK（差異はホールとして抽出）
                (DetailedNode::Literal(_), DetailedNode::Literal(_)) => true,
                // ノードタイプ: 一致必要
                (DetailedNode::NodeType(t1), DetailedNode::NodeType(t2)) => t1 == t2,
                _ => false,
            }
        },
        |edge_a, edge_b| edge_a == edge_b,  // エッジ種類は厳密一致
    )
}
```

## VF2アルゴリズムの利点と特徴

### 利点

1. **効率的な枝刈り**: 実行可能性ルールによる早期枝刈り
2. **汎用性**: 有向/無向グラフ、属性付きグラフに対応
3. **拡張性**: カスタム比較関数による柔軟なマッチング

### 計算量

- **最悪時**: O(N! × N) （Nは頂点数）
- **実用的**: 効率的な枝刈りにより、多くの実用的ケースで高速動作
- **グラフ同型判定問題**: NP完全ではないと考えられているが、多項式時間アルゴリズムは未発見

### 改良版アルゴリズム

- **VF2++**: ノード順序最適化と非再帰実装による高速化
- **ISMAGS**: 大規模グラフ向け部分グラフ同型判定

## RefSynにおける応用

### 操作パターンマッチング

```rust
// 例: 異なる順序の操作列が構造的に同等かを判定
let ops_a = vec![
    Op { id: "op1".to_string(), kind: OpKind::AddNode { id: "n1".to_string(), ... } },
    Op { id: "op2".to_string(), kind: OpKind::AddEdge { from: "n1".to_string(), to: "n2".to_string(), ... } }
];

let ops_b = vec![
    Op { id: "op_x".to_string(), kind: OpKind::AddEdge { from: "n_a".to_string(), to: "n_b".to_string(), ... } },
    Op { id: "op_y".to_string(), kind: OpKind::AddNode { id: "n_a".to_string(), ... } }
];

// VF2による構造的同型判定
assert!(match_graphs_with_flexible_structure(&ops_a, &ops_b));
```

この例では、操作の具体的なIDや順序が異なっていても、依存関係の構造が同じであれば同型と判定されます。

## まとめ

VF2アルゴリズムは、RefSynの操作パターンマッチングにおいて中核的な役割を果たしています。単純な線形比較では捉えられない構造的同等性を効率的に判定し、より柔軟で正確なプログラム合成を可能にしています。

## 参考文献

1. Luigi P. Cordella, Pasquale Foggia, Carlo Sansone, Mario Vento: "A (Sub)Graph Isomorphism Algorithm for Matching Large Graphs", IEEE Transactions on Pattern Analysis and Machine Intelligence, vol. 26, no. 10, pp. 1367-1372, Oct., 2004.

2. NetworkX Documentation: VF2 Algorithm - https://networkx.org/documentation/stable/reference/algorithms/isomorphism.vf2.html

3. petgraph Documentation: https://docs.rs/petgraph/latest/petgraph/algo/fn.is_isomorphic_matching.html
