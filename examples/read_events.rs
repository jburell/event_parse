extern crate event_parse;
extern crate itertools;
use itertools::Itertools;
use std::process;
use std::io::{Read, Write};

fn main() {
	match event_parse::list_devices() {
		Ok(devices) => {
			let num_devices = devices.len() as u32;
			devices.into_iter()
			.sorted_by(|a, b| Ord::cmp(&a.0, &b.0))
			.into_iter()
			.for_each(|device| {
				let dev = (device.1).0; // Ewwww....
				println!("{}: {:?}", device.0, dev);
			});

			print!("\nPick a device: ");
			std::io::stdout().flush().unwrap();

            let mut input_string = String::new();
			std::io::stdin()
                .read_to_string(&mut input_string)
                .unwrap();
            let c = input_string
                .trim()
                .chars()
                .next()
                .unwrap();
			
			if let Some(number) = c.to_digit(10) {
				match number {
					number if number > 0 && number < num_devices => {
						println!("You picked: {}", number);
					},
					_ => {
						println!("Please choose a valid device number between 0 and {}!", num_devices);
						process::exit(1);
					},
				}
			} else {
				println!("Please choose a valid device number between 0 and {}!", num_devices);
				process::exit(1);
			}
		},
		Err(e) => {
			println!("Could not access device list: {}", e);
			process::exit(1);
		},
	}
}
