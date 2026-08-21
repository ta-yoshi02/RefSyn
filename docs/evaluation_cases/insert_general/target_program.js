class Obj {
  constructor(f, g = null) {
    this.f = f;
    this.g = g;
  }

  // Oracle behavior represented by the three recorded operations.
  // Insert a new Obj with value b after the i-th node reachable through g.
  insert(i, b) {
    let h = this;
    let steps = i;
    while (steps > 0 && h !== null) {
      h = h.g;
      steps -= 1;
    }

    const next = h === null ? null : h.g;
    const tmp = new Obj(b);
    tmp.g = next;
    if (h !== null) {
      h.g = tmp;
    }
  }
}
