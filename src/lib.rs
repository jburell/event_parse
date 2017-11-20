extern crate libevdev_sys;
extern crate libc;
use self::libevdev_sys::evdev::*;
use self::libevdev_sys::linux_input::*;
use std::ptr;
use std::ffi::CStr;
use std::os::unix::io::IntoRawFd;
use std::fs::File;
use self::libc::timeval;

#[derive(Debug, Clone, Eq, PartialEq)]
enum EvdevType {
    EV_SYN,
    EV_KEY,
    EV_REL,
    EV_ABS,
    UNDEFINED(u16),
}

impl From<u16> for EvdevType {
    fn from(num: u16) -> Self {
        match num {
            0 => EvdevType::EV_SYN,
            1 => EvdevType::EV_KEY,
            2 => EvdevType::EV_REL,
            3 => EvdevType::EV_ABS,
            _ => EvdevType::UNDEFINED(num),
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
    SYN_REPORT,
    UNDEFINED(u16),
}

impl From<u16> for SynCode {
    fn from(num: u16) -> Self {
        match num {
            0 => SynCode::SYN_REPORT,
            _ => SynCode::UNDEFINED(num),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum AbsCode {
    ABS_X,
    ABS_Y,
    UNDEFINED(u16),
}

impl From<u16> for AbsCode {
    fn from(num: u16) -> Self {
        match num {
            0 => AbsCode::ABS_X,
            1 => AbsCode::ABS_Y,
            _ => AbsCode::UNDEFINED(num),
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
enum EvdevCode {
    SynCode(SynCode),
    AbsCode(AbsCode),
    UNDEFINED(u16),
}

impl From<(u16, u16)> for EvdevCode {
    fn from(type_and_num: (u16, u16)) -> Self {
        match EvdevType::from(type_and_num.0) {
            EvdevType::EV_SYN => EvdevCode::SynCode(SynCode::from(type_and_num.1)),
            EvdevType::EV_ABS => EvdevCode::AbsCode(AbsCode::from(type_and_num.1)),
            _ => EvdevCode::UNDEFINED(type_and_num.0),
        }
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct EvdevEvent {
    time: timeval,
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

        ev = ev.clone();

        match ret {
            r if r == (libevdev_read_status::LIBEVDEV_READ_STATUS_SUCCESS as i32) => {
                println!("[{}.{}] Code {:?}, Value {}",
                         ev.time.tv_sec,
                         ev.time.tv_usec,
                         EvdevCode::from((ev.type_, ev.code)),
                         ev.value);
                Ok(EvdevEvent {
                    time: ev.time,
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
        panic!("`libevdev_new_from_fd` failed: {}", ret);
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
        let expected = AbsCode::ABS_Y;
        if let EvdevCode::AbsCode(actual) = EvdevCode::from((3u16,1u16)) {
            assert_eq!(expected, actual);
        } else {
            panic!("Expected AbsCode");
        }
    }
}
