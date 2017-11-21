extern crate libevdev_sys;
extern crate libc;
use self::libevdev_sys::evdev::*;
use self::libevdev_sys::linux_input::*;
use std::{ptr, fmt};
use std::ffi::CStr;
use std::os::unix::io::IntoRawFd;
use std::fs::File;
//use self::libc::timeval;

#[derive(Clone, Copy)]
struct TimeVal {
	sec: i32,
	usec: i32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum EvdevType {
    EvSym,
    EvKey,
    EvRel,
    EvAbs,
   // UNDEFINED(u16),
}

impl From<u16> for EvdevType {
    fn from(num: u16) -> Self {
        match num {
            0 => EvdevType::EvSym,
            1 => EvdevType::EvKey,
            2 => EvdevType::EvRel,
            3 => EvdevType::EvAbs,
            _ => panic!(format!("EvdevType::UNDEFINED({})", num)),
        }
    }
}

//macro_rules! evdev_type {
//    ($tt, $expr) => (
//
//    )
//}
//
//evdev_type!(EV_SYN, 0);



#[derive(Debug, Clone, PartialEq)]
enum SynCode {
    SynReport,
   // UNDEFINED(u16),
}

impl From<u16> for SynCode {
    fn from(num: u16) -> Self {
        match num {
            0 => SynCode::SynReport,
            _ => panic!(format!("SynCode::UNDEFINED({})", num)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum KeyCode {
	BtnTouch,
	//Undefined(u16),
}

impl From<u16> for KeyCode {
	fn from(num: u16) -> Self {
		match num {
			330 => KeyCode::BtnTouch,
			_ => panic!(format!("KeyCode::Undefined({})", num)),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
enum AbsCode {
    AbsX,
    AbsY,
	AbsMtSlot,
	AbsMtPosX,
	AbsMtPosY,
	AbsMtTrackingId,
    //UNDEFINED(u16),
}

impl From<u16> for AbsCode {
    fn from(num: u16) -> Self {
        match num {
            0 => AbsCode::AbsX,
            1 => AbsCode::AbsY,
			47 => AbsCode::AbsMtSlot,
			53 => AbsCode::AbsMtPosX,
			54 => AbsCode::AbsMtPosY,
			57 => AbsCode::AbsMtTrackingId,
            _ => panic!(format!("AbsCode::UNDEFINED({})", num))
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
enum EvdevCode {
    SynCode(SynCode),
	KeyCode(KeyCode),
    AbsCode(AbsCode),
    //UNDEFINED(u16),
}

impl From<(u16, u16)> for EvdevCode {
    fn from(type_and_num: (u16, u16)) -> Self {
        match EvdevType::from(type_and_num.0) {
            EvdevType::EvSym => EvdevCode::SynCode(SynCode::from(type_and_num.1)),
            EvdevType::EvKey => EvdevCode::KeyCode(KeyCode::from(type_and_num.1)),
            EvdevType::EvAbs => EvdevCode::AbsCode(AbsCode::from(type_and_num.1)),
            _ => panic!(format!("EvdevCode::UNDEFINED({})", type_and_num.0)),
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
                /*println!("[{}.{}] Code {:?}, Value {}",
                         ev.time.tv_sec,
                         ev.time.tv_usec,
                         EvdevCode::from((ev.type_, ev.code)),
                         ev.value);*/
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

pub fn open_device(dev_nr: usize) -> Result<EventDevice, String> {
    let file = File::open(format!("/dev/input/event{}", dev_nr)).unwrap();
    let fd = file.into_raw_fd();

    let mut evdev: *mut libevdev = ptr::null_mut();
    let ret = unsafe { libevdev_new_from_fd(fd, &mut evdev) };
    if ret != 0 {
        panic!("libevdev_new_from_fd failed: {}", ret);
    }

    let name = unsafe { libevdev_get_name(evdev) };
    println!("device name: {}",
             unsafe { CStr::from_ptr(name) }.to_str().unwrap());

    Ok(EventDevice {
        stream: evdev,
        flags: libevdev_read_flag::LIBEVDEV_READ_FLAG_NORMAL as u32 |
               libevdev_read_flag::LIBEVDEV_READ_FLAG_BLOCKING as u32,
        ev: input_event::default(),
    })
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