// check.mjs
import assert from "node:assert/strict";

class Obj {
  constructor(v) {
    this.val = v;
    this.next = null;
  }

  // 実装1: 単一メソッド
  insert(i, arg) {
    let node = this;
    if(i === 0) {
      const added = new Obj(arg);
      added.next = node.next;
      node.next = added;
    } else {
        this.next.insert(i - 1, arg);
    }
  }

  // 実装2: 補助メソッドを使用
  insert_h0(i, arg, created0) { return arg;} 
  insert_h1(i, arg, created0, hole0) { 
        return (() => { 
            let __refsynCur = this; 
            let __refsynSteps = i; 
            if (!Number.isInteger(__refsynSteps) || __refsynSteps < 0) { 
                throw new RangeError("refsyn: nthNextRef needs a non-negative step count");
            } 
            while (__refsynSteps > 0 && __refsynCur !== null) {
                __refsynCur = ((__refsynValue) => { 
                    if (__refsynValue === undefined) { 
                        throw new TypeError("refsyn: read of an undefined field"); 
                    } return __refsynValue;
                })((__refsynCur).next);
                __refsynSteps -= 1;
            } 
            return __refsynCur; 
        })(); }
    insert_h2(i, arg, created0, hole0, hole1) { 
        return ((__refsynValue) => { 
            if (__refsynValue === undefined) { 
                throw new TypeError("refsyn: read of an undefined field"); 
            }
            return __refsynValue; 
        })((hole1).next); 
    } 
    insert_syn(i, arg) {
        const created0 = new Obj();
        const hole0 = this.insert_h0(i, arg, created0);
        const hole1 = this.insert_h1(i, arg, created0, hole0);
        const hole2 = this.insert_h2(i, arg, created0, hole0, hole1);
        created0.val = hole0;
        hole1.next = created0;
        created0.next = hole2;
    }
}

// 同じ初期状態を持つ独立したオブジェクトを用意
const obj1 = new Obj(10);
obj1.next = new Obj(20);
obj1.next.next = new Obj(30);

const obj2 = new Obj(10);
obj2.next = new Obj(20);
obj2.next.next = new Obj(30);

const operations = [
    [0, 15],
    [2, 25],
    [1, 35],
];

for(const [i, arg] of operations) {
    obj1.insert(i, arg);
    obj2.insert_syn(i, arg);
    assert.deepStrictEqual(obj1, obj2, `Mismatch after inserting ${arg} at position ${i}`);
}
