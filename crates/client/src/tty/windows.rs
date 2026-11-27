use super::{
    ENTER, LEAVE,
    console::{Handle, State, Writer, apply, mode, open},
    output::Output,
};
use std::{
    collections::VecDeque,
    io::{self, IsTerminal, Write},
};
use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, DISABLE_NEWLINE_AUTO_RETURN, ENABLE_ECHO_INPUT,
    ENABLE_EXTENDED_FLAGS, ENABLE_LINE_INPUT, ENABLE_MOUSE_INPUT, ENABLE_PROCESSED_INPUT,
    ENABLE_PROCESSED_OUTPUT, ENABLE_QUICK_EDIT_MODE, ENABLE_VIRTUAL_TERMINAL_INPUT,
    ENABLE_VIRTUAL_TERMINAL_PROCESSING, ENABLE_WINDOW_INPUT, GetConsoleCP, GetConsoleOutputCP,
    GetConsoleScreenBufferInfo, GetNumberOfConsoleInputEvents, INPUT_RECORD, KEY_EVENT,
    LEFT_CTRL_PRESSED, RIGHT_CTRL_PRESSED, ReadConsoleInputW, SetConsoleCP, SetConsoleOutputCP,
};

const UTF8: u32 = 65001;
const CLEAR: u32 = ENABLE_LINE_INPUT
    | ENABLE_ECHO_INPUT
    | ENABLE_PROCESSED_INPUT
    | ENABLE_QUICK_EDIT_MODE
    | ENABLE_MOUSE_INPUT;
const INPUT: u32 = ENABLE_VIRTUAL_TERMINAL_INPUT | ENABLE_WINDOW_INPUT | ENABLE_EXTENDED_FLAGS;
const OUTPUT: u32 =
    ENABLE_VIRTUAL_TERMINAL_PROCESSING | DISABLE_NEWLINE_AUTO_RETURN | ENABLE_PROCESSED_OUTPUT;

pub struct Terminal {
    input: Handle,
    screen: Handle,
    state: State,
    output: Output,
    pending: VecDeque<u8>,
    surrogate: Option<u16>,
    active: bool,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Err(io::Error::other(
                "interactive mode requires terminal stdin and stdout",
            ));
        }
        let input = open("CONIN$")?;
        let screen = open("CONOUT$")?;
        let state = State {
            input: mode(input.0)?,
            output: mode(screen.0)?,
            incp: unsafe { GetConsoleCP() },
            outcp: unsafe { GetConsoleOutputCP() },
        };
        let mut terminal = Self {
            input,
            screen,
            state,
            output: Output::default(),
            pending: VecDeque::new(),
            surrogate: None,
            active: true,
        };
        terminal.configure()?;
        terminal.control(ENTER)?;
        Ok(terminal)
    }

    pub fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.pending.is_empty() {
            self.poll()?;
        }
        let count = buffer.len().min(self.pending.len());
        for slot in &mut buffer[..count] {
            *slot = self.pending.pop_front().unwrap();
        }
        Ok(count)
    }

    pub fn size(&self) -> io::Result<(u16, u16)> {
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = unsafe { std::mem::zeroed() };
        if unsafe { GetConsoleScreenBufferInfo(self.screen.0, &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let columns = info
            .srWindow
            .Right
            .checked_sub(info.srWindow.Left)
            .and_then(|value| value.checked_add(1))
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| io::Error::other("invalid console width"))?;
        let rows = info
            .srWindow
            .Bottom
            .checked_sub(info.srWindow.Top)
            .and_then(|value| value.checked_add(1))
            .and_then(|value| u16::try_from(value).ok())
            .ok_or_else(|| io::Error::other("invalid console height"))?;
        Ok((columns, rows))
    }

    pub fn offer(&mut self, frame: Vec<u8>) -> io::Result<()> {
        self.output.offer(frame)
    }

    pub fn present(&mut self, paint: crate::Paint) -> io::Result<()> {
        self.output.present(paint)
    }

    pub fn copy(&mut self, copy: &crate::Copy) -> bool {
        self.output.copy(copy)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        let mut writer = Writer(self.screen.0);
        self.output.flush(&mut writer)
    }

    pub fn handle(&self) -> *mut std::ffi::c_void {
        self.input.0
    }

    pub fn ready(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn finish(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;
        let mut failures = Vec::new();
        let mut writer = Writer(self.screen.0);
        if let Err(error) = self.output.drain(&mut writer) {
            failures.push(format!(
                "output incomplete; copied effects unknown: {error}"
            ));
            if let Err(error) = self.control(b"\x18\x1b\\") {
                failures.push(format!("output cancellation: {error}"));
            }
        }
        if let Err(error) = self.control(LEAVE) {
            failures.push(format!("display: {error}"));
        }
        if let Err(error) = apply(self.input.0, self.state.input) {
            failures.push(format!("input mode: {error}"));
        }
        if let Err(error) = apply(self.screen.0, self.state.output) {
            failures.push(format!("output mode: {error}"));
        }
        if self.state.incp != 0 && unsafe { SetConsoleCP(self.state.incp) } == 0 {
            failures.push(format!("input codepage: {}", io::Error::last_os_error()));
        }
        if self.state.outcp != 0 && unsafe { SetConsoleOutputCP(self.state.outcp) } == 0 {
            failures.push(format!("output codepage: {}", io::Error::last_os_error()));
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(failures.join("; ")))
        }
    }

    fn configure(&self) -> io::Result<()> {
        if unsafe { SetConsoleCP(UTF8) } == 0 || unsafe { SetConsoleOutputCP(UTF8) } == 0 {
            return Err(io::Error::last_os_error());
        }
        apply(self.input.0, (self.state.input & !CLEAR) | INPUT)?;
        apply(self.screen.0, self.state.output | OUTPUT)
    }

    fn control(&mut self, bytes: &[u8]) -> io::Result<()> {
        let mut writer = Writer(self.screen.0);
        let mut offset = 0;
        while offset < bytes.len() {
            let count = writer.write(&bytes[offset..])?;
            if count == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "terminal output closed",
                ));
            }
            offset += count;
        }
        Ok(())
    }

    fn poll(&mut self) -> io::Result<()> {
        let mut available = 0u32;
        if unsafe { GetNumberOfConsoleInputEvents(self.input.0, &mut available) } == 0 {
            return Err(io::Error::last_os_error());
        }
        if available == 0 {
            return Ok(());
        }
        let mut records: [INPUT_RECORD; 64] = unsafe { std::mem::zeroed() };
        let wanted = (available as usize).min(records.len()) as u32;
        let mut read = 0u32;
        if unsafe { ReadConsoleInputW(self.input.0, records.as_mut_ptr(), wanted, &mut read) } == 0
        {
            return Err(io::Error::last_os_error());
        }
        for record in &records[..read as usize] {
            self.record(record);
        }
        Ok(())
    }

    fn record(&mut self, record: &INPUT_RECORD) {
        if record.EventType as u32 != KEY_EVENT {
            return;
        }
        let key = unsafe { record.Event.KeyEvent };
        if key.bKeyDown == 0 {
            return;
        }
        let unit = unsafe { key.uChar.UnicodeChar };
        let repeat = key.wRepeatCount.max(1);
        if unit == 0 {
            self.surrogate = None;
            let control = key.dwControlKeyState & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0;
            if key.wVirtualKeyCode == 0 || (control && matches!(key.wVirtualKeyCode, 0x20 | 0x32)) {
                for _ in 0..repeat {
                    self.pending.push_back(0);
                }
            }
            return;
        }
        self.decode(unit, repeat);
    }

    fn decode(&mut self, unit: u16, repeat: u16) {
        if (0xD800..=0xDBFF).contains(&unit) {
            self.surrogate = Some(unit);
            return;
        }
        let scalar = if (0xDC00..=0xDFFF).contains(&unit) {
            let Some(high) = self.surrogate.take() else {
                return;
            };
            let code = 0x10000 + (((high as u32 - 0xD800) << 10) | (unit as u32 - 0xDC00));
            char::from_u32(code)
        } else {
            self.surrogate = None;
            char::from_u32(unit as u32)
        };
        let Some(character) = scalar else {
            return;
        };
        let mut encoded = [0u8; 4];
        let slice = character.encode_utf8(&mut encoded);
        for _ in 0..repeat {
            self.pending.extend(slice.as_bytes());
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            let _ = writeln!(io::stderr(), "bagua terminal restoration failed: {error}");
        }
    }
}
