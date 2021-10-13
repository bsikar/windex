mod config;
mod windex;

use windex::Windex;

fn main() {
    let mut windex = Windex::new();

    windex.run();
}
