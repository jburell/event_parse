extern crate event_parse;

fn main() {
	let mut device = event_parse::open_device(1).unwrap();
	loop {
		println!("{:?}", device.read());
	}
}
