#![feature(try_trait)]
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate const_cstr;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod macros;
mod datis;
mod error;
mod srs;
mod station;
mod utils;

use std::ffi::CString;
use std::fs::File;
use std::ptr;

use crate::datis::Datis;
use hlua51::Lua;
use libc::c_int;
use lua51_sys as ffi;
use simplelog::*;

static mut DATIS: Option<Datis> = None;

#[no_mangle]
pub extern "C" fn init(state: *mut ffi::lua_State) -> c_int {
    unsafe {
        let received = ffi::lua_gettop(state);
        if received != 2 {
            // expect 2 argument
            return report_error(state, "expected 2 argument: cpath, logFile");
        }

        if ffi::lua_isstring(state, -2) == 0 {
            ffi::lua_pop(state, 2);
            return report_error(state, "argument cpath must be a string");
        }

        if ffi::lua_isstring(state, -1) == 0 {
            ffi::lua_pop(state, 2);
            return report_error(state, "argument logFile must be a string");
        }

        let lua = Lua::from_existing_state(state, false);
        let cpath = from_cstr!(ffi::lua_tostring(state, -2));
        let log_file = from_cstr!(ffi::lua_tostring(state, -1));
        ffi::lua_pop(state, 2); // remove arguments from stack

        CombinedLogger::init(vec![WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            // TODO: unwrap
            File::create(log_file.as_ref()).unwrap(),
        )])
            .unwrap();

        debug!("Initializing ...");

        match Datis::create(lua, cpath.into_owned()) {
            Ok(datis) => {
                DATIS = Some(datis);
            }
            Err(err) => {
                let msg = err.to_string();
                return report_error(state, &msg);
            }
        }

        0
    }
}

unsafe fn report_error(state: *mut ffi::lua_State, msg: &str) -> c_int {
    let msg = CString::new(msg).unwrap();

    ffi::lua_pushstring(state, msg.as_ptr());
    ffi::lua_error(state);

    0
}

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn luaopen_datis(state: *mut ffi::lua_State) -> c_int {
    let registration = &[
        ffi::luaL_Reg {
            name: cstr!("init"),
            func: Some(init),
        },
        ffi::luaL_Reg {
            name: ptr::null(),
            func: None,
        },
    ];

    ffi::luaL_openlib(state, cstr!("datis"), registration.as_ptr(), 0);

    1
}
