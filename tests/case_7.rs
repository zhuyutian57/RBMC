
fn main() {
  let mut a1 = [1, 2, 3 ,4, 5];
  a1[3] = 101;
  let b = &mut a1[1..2];
  b[0] = 111;
  let c = &mut a1[..2];
  c[1] = 123;
  let d = &mut a1[1..];
  d[3] = 100;
  let e = &mut a1[..];
  e[0] = 0;
  let f = &a1[3..4];
}