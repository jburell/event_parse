extern crate event_parse;

fn main() {
	let devices = event_parse::list_devices().unwrap();
	for device in devices {
		let mut dev = (device.1).0; // Ewwww....
		println!("{}: {:?}", device.0, dev);//dev.read();
	}
}
