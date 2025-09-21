// プログラム解析機能
// エディタ上のJavaScriptコードを解析してオブジェクトIDとメソッド呼び出しの関係を特定

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ProgramAnalysis {
    pub object_declarations: HashMap<String, String>, // var_name -> object_id
    pub method_calls: Vec<MethodCallInfo>,
}

#[derive(Debug, Clone)]
pub struct MethodCallInfo {
    pub receiver_var: String, // 例: "lst"
    pub receiver_id: String,  // 例: "main-new2"
    pub method_name: String,  // 例: "append"
    pub call_position: usize, // 呼び出し順序
}

impl ProgramAnalysis {
    pub fn new() -> Self {
        Self {
            object_declarations: HashMap::new(),
            method_calls: Vec::new(),
        }
    }

    /// JavaScriptプログラムを解析してオブジェクトIDを特定
    ///
    /// 例:
    /// ```javascript
    /// var list = new Node();  // main-new1
    /// var lst = new Node();   // main-new2
    /// lst.append(0);          // receiver = main-new2
    /// ```
    pub fn analyze_program(program_text: &str) -> Result<Self, String> {
        let mut analysis = Self::new();
        let mut object_counter = 1;

        // 簡単な正規表現ベースの解析（実際にはesprima等のパーサーを使用すべき）
        for line in program_text.lines() {
            let line = line.trim();

            // var xxx = new Node(); パターンを検出
            if let Some(var_name) = Self::extract_var_declaration(line) {
                let object_id = format!("main-new{}", object_counter);
                analysis.object_declarations.insert(var_name, object_id);
                object_counter += 1;
            }

            // xxx.method() パターンを検出
            if let Some((receiver_var, method_name)) = Self::extract_method_call(line) {
                if let Some(receiver_id) = analysis.object_declarations.get(&receiver_var) {
                    analysis.method_calls.push(MethodCallInfo {
                        receiver_var,
                        receiver_id: receiver_id.clone(),
                        method_name,
                        call_position: analysis.method_calls.len(),
                    });
                }
            }
        }

        Ok(analysis)
    }

    fn extract_var_declaration(line: &str) -> Option<String> {
        // var xxx = new Node(); パターンをマッチ
        if line.contains("var ") && line.contains("= new ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 && parts[0] == "var" && parts[2] == "=" {
                return Some(parts[1].to_string());
            }
        }
        None
    }

    fn extract_method_call(line: &str) -> Option<(String, String)> {
        // xxx.method(...) パターンをマッチ
        if let Some(dot_pos) = line.find('.') {
            if let Some(paren_pos) = line.find('(') {
                let receiver_var = line[..dot_pos].trim().to_string();
                let method_name = line[dot_pos + 1..paren_pos].trim().to_string();
                return Some((receiver_var, method_name));
            }
        }
        None
    }

    /// 特定のメソッド呼び出しのreceiver_idを取得
    pub fn get_receiver_id_for_call(&self, call_index: usize) -> Option<&str> {
        self.method_calls
            .get(call_index)
            .map(|call| call.receiver_id.as_str())
    }

    /// Create environment mapping that maps main-new* IDs to 'this' for method calls
    /// This allows tests to dynamically determine which object ID corresponds to 'this'
    pub fn create_environment_mapping(&self) -> HashMap<String, String> {
        let mut mapping = HashMap::new();

        // For each method call, map the receiver's object ID to 'this'
        for method_call in &self.method_calls {
            mapping.insert(method_call.receiver_id.clone(), "this".to_string());
        }

        mapping
    }

    /// Get the object ID for a specific receiver variable
    pub fn get_object_id_for_receiver(&self, receiver_variable: &str) -> Option<String> {
        self.object_declarations.get(receiver_variable).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_analysis() {
        let program = r#"
            class Node {
                append(arg) { /* TBD */ }
            }
            var list = new Node();
            var lst = new Node();
            lst.val = 2;
            lst.append(0);
            lst.append(3);
        "#;

        let analysis = ProgramAnalysis::analyze_program(program).unwrap();

        // オブジェクト宣言の確認
        assert_eq!(
            analysis.object_declarations.get("list"),
            Some(&"main-new1".to_string())
        );
        assert_eq!(
            analysis.object_declarations.get("lst"),
            Some(&"main-new2".to_string())
        );

        // メソッド呼び出しの確認
        assert_eq!(analysis.method_calls.len(), 2);
        assert_eq!(analysis.method_calls[0].receiver_id, "main-new2");
        assert_eq!(analysis.method_calls[0].method_name, "append");
        assert_eq!(analysis.method_calls[1].receiver_id, "main-new2");
        assert_eq!(analysis.method_calls[1].method_name, "append");
    }
}
