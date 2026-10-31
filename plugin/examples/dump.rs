// Print the content stream of one page, for `make baseline` and `make check`, which
// compare a change to the plugin against the pages before it, byte for byte:
//   cargo run --release --example dump -- grown 7 [spilly] [age]
// MARKS="h1@20.5,h2@14,b@9,blily@6" adds marks (heading levels, bloom(motif)) at
// those heights (cm); FRAME="k/n" or "k/n/explain" writes frame k of n of the growth.
use florilegium::{authored, grow, ink};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let mode = a.get(1).map_or("grown", |s| s.as_str());
    let fixed: f64 = a.get(2).map_or(7.0, |s| s.parse().unwrap());
    let spilly: f64 = a.get(3).map_or(0.0, |s| s.parse().unwrap());
    let age: Option<f64> = a.get(4).map(|s| s.parse().unwrap());
    let geo = grow::Geo {
        pw: 21.0, ph: 29.7, textleft: 6.8, textright: 18.8, ytop: 26.5, ybot: 3.0,
        colright: 6.4, headrule: 27.2, footy: 3.0 - 30.0 * 2.54 / 72.27,
    };
    let seed = fixed * 7919.0 + 131.0;
    let marks = std::env::var("MARKS")
        .unwrap_or_default()
        .split(',')
        .filter(|m| !m.is_empty())
        .map(|m| {
            let (what, y) = m.split_once('@').unwrap();
            let y = y.parse().unwrap();
            match what.strip_prefix('h') {
                Some(level) => grow::Mark { heading: Some(level.parse().unwrap()), motif: String::new(), y },
                None => grow::Mark { heading: None, motif: what[1..].to_string(), y },
            }
        })
        .collect();
    let frame = std::env::var("FRAME").ok();
    ink::reset();
    if mode == "authored" {
        authored::page(&geo, seed, true);
        print!("{}", ink::flush().trim_end());
        return;
    }
    let out = grow::page(&grow::Input { geo, seed, spill: spilly > 0.0, spilly, visitors: true, age, marks, anim: frame.is_some() });
    match (frame, out.rec) {
        (Some(f), Some((rec, events, total))) => {
            let p: Vec<&str> = f.split('/').collect();
            let (k, n): (usize, usize) = (p[0].parse().unwrap(), p[1].parse().unwrap());
            let explain = p.get(2) == Some(&"explain");
            let t = if k >= n { f64::INFINITY } else { total * (k as f64 - 1.0) / (n.max(2) - 1) as f64 };
            print!("{}", ink::emit(&rec, |tag| {
                if tag.overlay && !explain {
                    return false;
                }
                let e = &events[tag.ev];
                let p = (t - e.t0.unwrap()) / e.dur;
                tag.visible(p)
            }).trim_end());
        }
        _ => print!("{}", ink::flush().trim_end()),
    }
}
