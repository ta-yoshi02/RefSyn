# 統合されたSynthesizeエンドポイント

## 概要

`/synthesize`エンドポイントが拡張され、複数の操作列が提供された場合に自動的に操作分析機能も実行するようになりました。これにより、単一のエンドポイントでコード合成と操作差異分析の両方が利用できます。

## エンドポイント

```
POST http://127.0.0.1:3030/synthesize
```

## リクエスト形式

```json
{
  "method_calls": [
    {
      "callLabel": "method1",
      "contextSensitiveID": "ctx1",
      "receiverObject": "obj1",
      "methodName": "test",
      "operations": [
        {
          "edit_type": "addNode",
          "node_id": "n3",
          "node_data": {"type": "Node"}
        },
        {
          "edit_type": "addEdge",
          "from": "n1",
          "to": "n3",
          "edge_data": {"type": "Edge"}
        }
      ]
    },
    {
      "callLabel": "method2",
      "contextSensitiveID": "ctx2", 
      "receiverObject": "obj2",
      "methodName": "test",
      "operations": [
        {
          "edit_type": "addNode",
          "node_id": "n3",
          "node_data": {"type": "Node"}
        },
        {
          "edit_type": "addEdge",
          "from": "n2",
          "to": "n3",
          "edge_data": {"type": "Edge"}
        }
      ]
    }
  ],
  "vis_graph": {
    "nodes": [
      {"id": "n1", "is_literal": false, "label": "node1"},
      {"id": "n2", "is_literal": false, "label": "node2"}
    ],
    "edges": [
      {"from": "n1", "to": "n2", "label": "edge1"}
    ]
  }
}
```

## レスポンス形式

### 単一操作列の場合
```json
{
  "common_pattern": "...",
  "hole_information": {...},
  "code": [...],
  "individual_codes": [...],
  "list_environment_info": "...",
  "operation_analysis": null
}
```

### 複数操作列の場合（操作分析含む）
```json
{
  "common_pattern": "...",
  "hole_information": {...},
  "code": [...],
  "individual_codes": [...],
  "list_environment_info": "...",
  "operation_analysis": {
    "common_operations_count": 1,
    "total_operations_counts": [2, 2],
    "differences": [
      "Position 1: Some(GraphOperation { edit_type: \"addEdge\", from: Some(\"n1\"), to: Some(\"n3\"), ... }) vs Some(GraphOperation { edit_type: \"addEdge\", from: Some(\"n2\"), to: Some(\"n3\"), ... })"
    ],
    "list_environments_at_differences": [
      "Environment at position 1:\nNodes: {\"n1\": 0, \"n2\": 1, \"n3\": 2}\n..."
    ]
  }
}
```

## 機能

1. **コード合成**: 従来の合成機能（共通パターンの抽出、ホール情報の生成）
2. **List環境生成**: VisGraphからListEnvironmentへの変換
3. **操作分析**: 複数操作列が提供された場合、自動的に差異分析を実行
   - 共通操作の数
   - 各操作列の操作数
   - 差異点の詳細
   - 各差異点でのList環境

## 使用例

### curl を使用した例

```bash
curl -X POST http://127.0.0.1:3030/synthesize \
  -H "Content-Type: application/json" \
  -d @request.json
```

### JavaScript/Kanonからの使用

```javascript
fetch('http://127.0.0.1:3030/synthesize', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
  },
  body: JSON.stringify({
    method_calls: [
      // 操作列1
      {
        callLabel: "addNodeAndConnect1",
        contextSensitiveID: "ctx1",
        receiverObject: "graph1",
        methodName: "modify",
        operations: [...] // Kanon operations
      },
      // 操作列2  
      {
        callLabel: "addNodeAndConnect2",
        contextSensitiveID: "ctx2",
        receiverObject: "graph2", 
        methodName: "modify",
        operations: [...] // 異なるKanon operations
      }
    ],
    vis_graph: kanonVisGraph
  })
})
.then(response => response.json())
.then(data => {
  // data.common_pattern: 合成されたパターン
  // data.operation_analysis: 操作分析結果（複数操作列の場合）
  // data.list_environment_info: List環境情報
});
```

## 変更点

- `/analyze`エンドポイントを削除
- `/synthesize`エンドポイントに操作分析機能を統合
- `operation_analysis`フィールドを`SynthesisResponse`に追加
- 複数操作列が提供された場合のみ操作分析を実行

## 利点

1. **統一API**: 単一エンドポイントでの完全な機能提供
2. **効率性**: 共通部分処理の重複排除
3. **簡潔性**: クライアント側の実装が簡素化
4. **自動判定**: 操作列の数に基づく自動的な機能切り替え
