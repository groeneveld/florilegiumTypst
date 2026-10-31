// Time the plugin's page function natively: cargo run --release --example bench [n]
fn main() {
    let n: usize = std::env::args().nth(1).map_or(20, |s| s.parse().unwrap());
    let json = r#"{"geo": {"pw": 21, "ph": 29.7, "textleft": 6.8, "textright": 18.8, "ytop": 26.5, "ybot": 3}, "page": 1, "seed": 7}"#;
    let t = std::time::Instant::now();
    for _ in 0..n {
        florilegium::page(json.as_bytes()).unwrap();
    }
    println!("{:.1} ms a page", t.elapsed().as_secs_f64() * 1000.0 / n as f64);
}
