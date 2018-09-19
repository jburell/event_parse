extern crate libevdev_sys;
extern crate libc;
#[macro_use] extern crate custom_derive;
#[macro_use] extern crate newtype_derive;
#[macro_use] extern crate enum_primitive;
extern crate num;
use num::FromPrimitive;

use self::libevdev_sys::evdev::*;
use self::libevdev_sys::linux_input::*;
use std::{ptr, fmt};
use std::ffi::CStr;
use std::os::unix::io::IntoRawFd;
use std::fs::{File, self};
use std::collections::HashMap;

mod codes;

#[cfg(target_pointer_width = "32")]
type Int = i32;

#[cfg(target_pointer_width = "64")]
type Int = i64;

#[repr(C)]
#[derive(Clone, Copy)]
struct TimeVal {
	sec: Int,
	usec: Int,
}

enum_from_primitive! {
    #[derive(Debug, Clone, Eq, PartialEq)]
    enum EvdevType {
        EvSym = 0,
        EvKey = 1,
        EvRel = 2,
        EvAbs = 3,
    }
}

enum_from_primitive! {
    #[derive(Debug, Clone, PartialEq)]
    enum SynCode {
        SynReport = 0,
    }
}

enum_from_primitive! {
    #[derive(Debug, Clone, PartialEq)]
    enum KeyCode {
        BtnTouch = 330,
    }
}

enum_from_primitive! {
    #[derive(Debug, Clone, PartialEq)]
    enum AbsCode {
        AbsX             = 0,
        AbsY             = 1,
        AbsMtSlot        = 47,
        AbsMtPosX        = 53,
        AbsMtPosY        = 54,
        AbsMtTrackingId  = 57,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum EvdevCode {
    SynCode(SynCode),
	KeyCode(KeyCode),
    AbsCode(AbsCode),
    Undefined(u16),
}

impl From<(u16, u16)> for EvdevCode {
    fn from(type_and_num: (u16, u16)) -> Self {
        match EvdevType::from_u16(type_and_num.0 as _) {
            // TODO: Remove unwraps
            Some(EvdevType::EvSym) => EvdevCode::SynCode(SynCode::from_u16(type_and_num.1).unwrap()),
            Some(EvdevType::EvKey) => if let Some(kc) = KeyCode::from_u16(type_and_num.1) {
						EvdevCode::KeyCode(kc)
					} else {
						println!("Unknown key code: {:?}", type_and_num);
						EvdevCode::Undefined(type_and_num.0)
					}
            Some(EvdevType::EvAbs) => if let Some(ac) = AbsCode::from_u16(type_and_num.1) {
						EvdevCode::AbsCode(ac)
					} else {
						println!("Unknown abs code: {:?}", type_and_num);
						EvdevCode::Undefined(type_and_num.0)
					}
            //_ => panic!(format!("EvdevCode::Undefined({:x})", type_and_num.0)),
            _ => {
			println!("EvdevCode::Undefined({:x})", type_and_num.0);
			EvdevCode::Undefined(type_and_num.0)
		}
        }
    }
}

#[derive(Clone, Debug)]
pub struct EvdevData {
    code: EvdevCode,
    val: i32,
}

impl From<input_event> for EvdevData {
    fn from(ev: input_event) -> Self {
        EvdevData {
            code: EvdevCode::from((ev.type_, ev.code)),
            val: ev.value,
        }
    }
}

impl fmt::Debug for TimeVal {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "[{},{}]", self.sec, self.usec)
	}
}

#[derive(Clone, Debug)]
pub struct EvdevEvent {
    time: TimeVal,
    ev: EvdevData,
}

pub struct EventDevice {
    stream: *mut libevdev,
    flags: u32,
    ev: input_event,
}

impl EventDevice {
    pub fn read_name(&mut self) {}

    pub fn read(&mut self) -> Result<EvdevEvent, String> {
        let mut ev = input_event::default();
        let ret = unsafe { libevdev_next_event(self.stream, self.flags, &mut ev) };

        self.ev = ev.clone();

        match ret {
            r if r == (libevdev_read_status::LIBEVDEV_READ_STATUS_SUCCESS as i32) => {
                Ok(EvdevEvent {
                    time: TimeVal {
									sec: ev.time.tv_sec,
									usec: ev.time.tv_usec,
								  },
                    ev: ev.into(),
                })
            }
            r if r == (libevdev_read_status::LIBEVDEV_READ_STATUS_SYNC as i32) => {
                Err("SYNC!".to_string())
            }
            r if r == -libc::EAGAIN => {
                // No events available, sleep and loop
                //sleep(Duration::from_millis(20));
                Err("Empty".to_string())
            }
            _ => Err(format!("failed to read event: {}", ret)),
        }
    }
}

custom_derive! {
    #[derive(Debug, NewtypeDisplay, NewtypeFrom)]
    pub struct Error(String);
}

impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error(err.to_string())
    }
}

impl std::convert::From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error(err.to_string())
    }
}

pub fn list_devices() -> Result<HashMap<usize, (String, EventDevice)>, Error> {    
    let mut devices = HashMap::new();
    for entry in fs::read_dir("/dev/input")? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            continue;
        }

        // TODO: Remove unrwap
        let path = path.file_name().unwrap().to_str().unwrap();
        if !path.starts_with("event") {
            continue;
        }
        if path.len() > "event".len() {
            let num_str: &str = path.split_at("event".len()).1;
            let num = num_str.to_string().parse::<usize>()?;
            let dev = get_device_from_idx(num)?;
            let name = get_name_from_device(&dev).to_string();
            devices.insert(num, (name, dev));
        }
    }
    Ok(devices)
} 

/*fn print_props(dev: &Device) {
	println!("Properties:");

	for input_prop in InputProp::INPUT_PROP_POINTER.iter() {
		if dev.has(&input_prop) {
			println!("  Property type: {}", input_prop);
        }
    }
}*/


fn get_device_from_idx(idx: usize) -> Result<EventDevice, Error> {
    let file = File::open(format!("/dev/input/event{}", idx))?;
    let fd = file.into_raw_fd();

    let mut evdev: *mut libevdev = ptr::null_mut();
    let ret = unsafe { libevdev_new_from_fd(fd, &mut evdev) };
    if ret != 0 {
        return Err(Error(format!("libevdev_new_from_fd failed: {}", ret)));
    }

    Ok(EventDevice {
        stream: evdev,
        flags: libevdev_read_flag::LIBEVDEV_READ_FLAG_NORMAL as u32 |
               libevdev_read_flag::LIBEVDEV_READ_FLAG_BLOCKING as u32,
        ev: input_event::default(),
    })
}

fn get_name_from_device(dev: &EventDevice) -> &'static str {
    let name = unsafe { libevdev_get_name(dev.stream) };
    unsafe { CStr::from_ptr(name) }.to_str().unwrap()
}

pub fn open_device(dev_nr: usize) -> Result<EventDevice, Error> {
    let device = get_device_from_idx(dev_nr)?;
    println!("device name: {}", get_name_from_device(&device));

    Ok(device)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_event_test() {
        let expected = AbsCode::AbsY;
        if let EvdevCode::AbsCode(actual) = EvdevCode::from((3u16,1u16)) {
            assert_eq!(expected, actual);
        } else {
            panic!("Expected AbsCode");
        }
    }
}
