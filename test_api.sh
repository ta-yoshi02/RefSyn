#!/bin/bash

# RefSyn操作解析API のテスト

echo "🔍 RefSyn操作解析APIテスト"
echo "=========================="

# テスト用のJSONペイロード
cat > test_payload.json << 'EOF'
{
  "vis_graph": {
    "nodes": [
      {
        "id": "main-new1",
        "is_literal": false,
        "label": "Node"
      },
      {
        "id": "main-new1-val",
        "is_literal": true,
        "label": "2"
      }
    ],
    "edges": [
      {
        "from": "main-new1",
        "to": "main-new1-val",
        "label": "val"
      }
    ]
  },
  "operations_list": [
    [
      {"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false},
      {"editType": "addNode", "id": "__temp2", "label": "0", "isLiteral": true},
      {"editType": "addEdge", "from": "__temp1", "to": "__temp2", "label": "val"},
      {"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}
    ],
    [
      {"editType": "addNode", "id": "__temp1", "label": "Node", "isLiteral": false},
      {"editType": "addNode", "id": "__temp3", "label": "3", "isLiteral": true},
      {"editType": "addEdge", "from": "__temp1", "to": "__temp3", "label": "val"},
      {"editType": "addEdge", "from": "main-new1", "to": "__temp1", "label": "next"}
    ]
  ]
}
EOF

echo "📤 APIリクエストを送信中..."

# curlでAPIを呼び出し
curl -X POST http://127.0.0.1:3030/analyze \
  -H "Content-Type: application/json" \
  -d @test_payload.json \
  | jq '.'

echo ""
echo "✅ テスト完了"

# 一時ファイルを削除
rm test_payload.json
