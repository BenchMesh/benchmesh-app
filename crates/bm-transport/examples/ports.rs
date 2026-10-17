//! Print the USB ports instruments can be identified on: `cargo run -p bm-transport --example ports`.
fn main() {
    for p in bm_transport::discover::ports() {
        println!("{:<7} {:<24} {:?}", p.kind.as_str(), p.path, p.usb);
    }
}
