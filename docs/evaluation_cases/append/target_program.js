class Obj {
  constructor(f, g = null) {
    this.f = f;
    this.g = g;
  }

  // Oracle behavior represented by the two recorded operations.
  append(x) {
    let tail = this;
    while (tail !== null && tail.g !== null) {
      tail = tail.g;
    }

    const tmp = new Obj(x);
    if (tail !== null) {
      tail.g = tmp;
    }
  }
}
