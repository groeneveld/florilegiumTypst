// Write the plugin's PDF for one page, from the same JSON Typst sends:
//   cargo run --release --example pagepdf -- '{"geo": {...}, "page": 1, "seed": 7}' out.pdf
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let pdf = florilegium::page(a[1].as_bytes()).unwrap();
    std::fs::write(&a[2], pdf).unwrap();
}
