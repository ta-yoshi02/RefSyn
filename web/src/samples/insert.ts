import type { SynthesisRequest } from "../lib/browser-runner";

// The paper's insert(i, arg) demonstrations, as Kanon records them (json/refsyn-insert-request.json).
export const insertSample: SynthesisRequest = {
  "method_calls": [
    {
      "callLabel": "call1",
      "contextSensitiveID": "main",
      "receiverObject": "main-new1",
      "receiverClassName": "Node",
      "methodName": "insert",
      "methodParamNames": [
        "i",
        "arg"
      ],
      "arguments": [
        0,
        89
      ],
      "operations": [
        {
          "editType": "addNode",
          "id": "__temp1",
          "isLiteral": false,
          "label": "Node"
        },
        {
          "editType": "addNode",
          "id": "__temp2",
          "isLiteral": true,
          "label": "89",
          "type": "number"
        },
        {
          "editType": "addEdge",
          "from": "__temp1",
          "label": "val",
          "to": "__temp2"
        },
        {
          "editType": "editEdgeReference",
          "from": "main-new1",
          "label": "next",
          "oldTo": "main-new2",
          "newTo": "__temp1"
        },
        {
          "editType": "addEdge",
          "from": "__temp1",
          "label": "next",
          "to": "main-new2"
        }
      ]
    },
    {
      "callLabel": "call2",
      "contextSensitiveID": "main",
      "receiverObject": "main-new1",
      "receiverClassName": "Node",
      "methodName": "insert",
      "methodParamNames": [
        "i",
        "arg"
      ],
      "arguments": [
        2,
        46
      ],
      "operations": [
        {
          "editType": "addNode",
          "id": "__temp3",
          "isLiteral": false,
          "label": "Node"
        },
        {
          "editType": "addNode",
          "id": "__temp4",
          "isLiteral": true,
          "label": "46",
          "type": "number"
        },
        {
          "editType": "addEdge",
          "from": "__temp3",
          "label": "val",
          "to": "__temp4"
        },
        {
          "editType": "editEdgeReference",
          "from": "main-new2",
          "label": "next",
          "oldTo": "main-new3",
          "newTo": "__temp3"
        },
        {
          "editType": "addEdge",
          "from": "__temp3",
          "label": "next",
          "to": "main-new3"
        }
      ]
    },
    {
      "callLabel": "call3",
      "contextSensitiveID": "main",
      "receiverObject": "main-new1",
      "receiverClassName": "Node",
      "methodName": "insert",
      "methodParamNames": [
        "i",
        "arg"
      ],
      "arguments": [
        1,
        71
      ],
      "operations": [
        {
          "editType": "addNode",
          "id": "__temp5",
          "isLiteral": false,
          "label": "Node"
        },
        {
          "editType": "addEdge",
          "from": "__temp5",
          "label": "next",
          "to": "main-new2"
        },
        {
          "editType": "editEdgeReference",
          "from": "__temp1",
          "label": "next",
          "oldTo": "main-new2",
          "newTo": "__temp5"
        },
        {
          "editType": "addNode",
          "id": "__temp6",
          "isLiteral": true,
          "label": "71",
          "type": "number"
        },
        {
          "editType": "addEdge",
          "from": "__temp5",
          "label": "val",
          "to": "__temp6"
        }
      ]
    }
  ],
  "vis_graph": {
    "nodes": [],
    "edges": []
  },
  "validation": {
    "version": 3,
    "classes": [
      {
        "name": "Node",
        "source": "class Node {\n    constructor() {\n        this.val = undefined;\n        this.next = null;\n    }\n\n    insert(i, arg) {\n    }\n}",
        "instance": {
          "val": {
            "undefined": true
          },
          "next": null
        }
      }
    ],
    "cases": [
      {
        "callLabel": "call1",
        "contextSensitiveID": "main",
        "objects": [
          {
            "id": "main-new1",
            "className": "Node",
            "fields": {
              "val": 39,
              "next": {
                "ref": "main-new2"
              }
            }
          },
          {
            "id": "main-new2",
            "className": "Node",
            "fields": {
              "val": 27,
              "next": {
                "ref": "main-new3"
              }
            }
          },
          {
            "id": "main-new3",
            "className": "Node",
            "fields": {
              "val": 53,
              "next": null
            }
          }
        ],
        "values": {
          "main-new1-val": 39,
          "main-new2-val": 27,
          "main-new3-val": 53
        },
        "arguments": [
          0,
          89
        ]
      },
      {
        "callLabel": "call2",
        "contextSensitiveID": "main",
        "objects": [
          {
            "id": "main-new1",
            "className": "Node",
            "fields": {
              "val": 39,
              "next": {
                "ref": "__temp1"
              }
            }
          },
          {
            "id": "__temp1",
            "className": "Node",
            "fields": {
              "val": 89,
              "next": {
                "ref": "main-new2"
              }
            }
          },
          {
            "id": "main-new2",
            "className": "Node",
            "fields": {
              "val": 27,
              "next": {
                "ref": "main-new3"
              }
            }
          },
          {
            "id": "main-new3",
            "className": "Node",
            "fields": {
              "val": 53,
              "next": null
            }
          }
        ],
        "values": {
          "main-new1-val": 39,
          "__temp1-val": 89,
          "main-new2-val": 27,
          "main-new3-val": 53
        },
        "arguments": [
          2,
          46
        ]
      },
      {
        "callLabel": "call3",
        "contextSensitiveID": "main",
        "objects": [
          {
            "id": "main-new1",
            "className": "Node",
            "fields": {
              "val": 39,
              "next": {
                "ref": "__temp1"
              }
            }
          },
          {
            "id": "__temp1",
            "className": "Node",
            "fields": {
              "val": 89,
              "next": {
                "ref": "main-new2"
              }
            }
          },
          {
            "id": "main-new2",
            "className": "Node",
            "fields": {
              "val": 27,
              "next": {
                "ref": "__temp3"
              }
            }
          },
          {
            "id": "__temp3",
            "className": "Node",
            "fields": {
              "val": 46,
              "next": {
                "ref": "main-new3"
              }
            }
          },
          {
            "id": "main-new3",
            "className": "Node",
            "fields": {
              "val": 53,
              "next": null
            }
          }
        ],
        "values": {
          "main-new1-val": 39,
          "__temp1-val": 89,
          "main-new2-val": 27,
          "__temp3-val": 46,
          "main-new3-val": 53
        },
        "arguments": [
          1,
          71
        ]
      }
    ]
  }
};
