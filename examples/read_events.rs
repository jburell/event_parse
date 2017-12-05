extern crate event_parse;

fn main() {
	match event_parse::list_devices() {
		Ok(devices) => devices.into_iter().for_each(|device| {
			let dev = (device.1).0; // Ewwww....
			println!("{}: {:?}", device.0, dev);
		}),
		Err(e) => println!("Could not access device list: {}", e),
	}
}
