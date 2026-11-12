// 縦の段階1: demonstration::read が、Kanon 形式の JSON (01-setval.json) を受け取って何を返すか。
// 今は表示するだけ。出力を予想してから実行し、確かめたことを assert に書き足していく。
// 実行: cargo test --test step_read -- --nocapture
use refsyn::demonstration;
use refsyn::SynthesisRequest;

#[test]
fn read_setval() {
    // 1. 手で書いた記録を、Kanon から届いたリクエストとして読み込む。
    let request: SynthesisRequest =
        serde_json::from_str(include_str!("steps/01-setval.json")).unwrap();

    // 2. 合成器の最初の段階。ここで JSON が Rust の型 (Recording) になる。
    let recording = demonstration::read(&request).unwrap();

    // 3. 中身を全部表示する。
    println!("{recording:#?}");
}
