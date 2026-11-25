use super::{
    Process,
    env::{block, line, wide},
    handle::{Console, Handle, Security},
    pipe::pair,
    reader::Reader,
    writer::Writer,
};
use crate::{CHUNK, Failure, Launch, Size, Stage};
use std::{
    ffi::c_void,
    io,
    mem::{size_of, zeroed},
    ptr,
    sync::atomic::AtomicU32,
};
use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::{
        Console::{COORD, CreatePseudoConsole},
        Threading::{
            CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DeleteProcThreadAttributeList,
            EXTENDED_STARTUPINFO_PRESENT, InitializeProcThreadAttributeList,
            LPPROC_THREAD_ATTRIBUTE_LIST, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, PROCESS_INFORMATION,
            STARTF_USESTDHANDLES, STARTUPINFOEXW, UpdateProcThreadAttribute,
        },
    },
};

struct Attributes {
    storage: Vec<u8>,
    ready: bool,
}

impl Attributes {
    fn new(console: &Console) -> io::Result<Self> {
        let mut size = 0usize;
        unsafe {
            InitializeProcThreadAttributeList(ptr::null_mut(), 1, 0, &mut size);
        }
        if size == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut attributes = Self {
            storage: vec![0u8; size],
            ready: false,
        };
        let list = attributes.raw();
        if unsafe { InitializeProcThreadAttributeList(list, 1, 0, &mut size) } == 0 {
            return Err(io::Error::last_os_error());
        }
        attributes.ready = true;
        let ok = unsafe {
            UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                console.raw() as *const c_void,
                size_of::<windows_sys::Win32::System::Console::HPCON>(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(attributes)
    }

    fn raw(&mut self) -> LPPROC_THREAD_ATTRIBUTE_LIST {
        self.storage.as_mut_ptr().cast()
    }
}

impl Drop for Attributes {
    fn drop(&mut self) {
        if self.ready {
            unsafe {
                DeleteProcThreadAttributeList(self.raw());
            }
        }
    }
}

pub(super) fn coord(size: Size) -> COORD {
    COORD {
        X: size.columns() as i16,
        Y: size.rows() as i16,
    }
}

pub(super) fn pack(size: Size) -> u32 {
    ((size.columns() as u32) << 16) | (size.rows() as u32)
}

pub(super) fn start(launch: Launch) -> Result<Process, Failure> {
    launch.check()?;
    let mut security = Security::private().map_err(|error| Failure::io(Stage::Open, error))?;
    let (write, input) = pair(false, CHUNK as u32, &mut security)
        .map_err(|error| Failure::io(Stage::Open, error))?;
    let (read, output) =
        pair(true, 65_536, &mut security).map_err(|error| Failure::io(Stage::Open, error))?;
    let mut raw = 0;
    let status =
        unsafe { CreatePseudoConsole(coord(launch.size), input.raw(), output.raw(), 0, &mut raw) };
    drop(input);
    drop(output);
    if status != 0 {
        return Err(Failure::io(
            Stage::Open,
            io::Error::from_raw_os_error(status),
        ));
    }
    let console = Console::new(raw);
    let mut attributes =
        Attributes::new(&console).map_err(|error| Failure::io(Stage::Open, error))?;
    let mut startup: STARTUPINFOEXW = unsafe { zeroed() };
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.lpAttributeList = attributes.raw();
    let program = wide(launch.program.as_os_str());
    let mut command = line(&launch.program, &launch.args);
    let mut environment = block(&launch.environment);
    let cwd = wide(launch.cwd.as_os_str());
    let mut info: PROCESS_INFORMATION = unsafe { zeroed() };
    let flags = EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT;
    let ok = unsafe {
        CreateProcessW(
            program.as_ptr(),
            command.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            flags,
            environment.as_mut_ptr().cast(),
            cwd.as_ptr(),
            &startup.StartupInfo,
            &mut info,
        )
    };
    drop(attributes);
    if ok == 0 {
        return Err(Failure::io(Stage::Launch, io::Error::last_os_error()));
    }
    unsafe {
        CloseHandle(info.hThread);
    }
    let child = Handle::new(info.hProcess).map_err(|error| Failure::io(Stage::Launch, error))?;
    let reader = Reader::new(read).map_err(|error| Failure::io(Stage::Open, error))?;
    let writer = Writer::new(write).map_err(|error| Failure::io(Stage::Open, error))?;
    Ok(Process {
        console: Some(console),
        reader: Some(reader),
        writer: Some(writer),
        child: Some(child),
        id: info.dwProcessId,
        size: AtomicU32::new(pack(launch.size)),
        exit: None,
        deadline: None,
        forced: false,
    })
}
