use std::io::{stdin, stdout, ErrorKind, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::panic::catch_unwind;

extern crate termios;
use termios::Termios;

extern crate libc;
use libc::ioctl;
// extern crate nix;
// use nix::sys::ioctl;

// *** Defines ***
struct Editor {
    orig_termios: Termios,
    stdin_fileno: RawFd,
    stdout_fileno: RawFd,
    screen_rows: u16,
    screen_cols: u16,
}

impl Editor {
    pub fn new() -> Editor {
        let mut editor = Editor {
            orig_termios: Termios::from_fd(stdin().as_raw_fd()).unwrap(),
            stdin_fileno: stdin().as_raw_fd(),
            stdout_fileno: stdout().as_raw_fd(),
            screen_rows: 0,
            screen_cols: 0,
        };
        editor.clear_screen();
        editor.get_window_size();
        println!(
            "Window Rows: {0}, Cols: {1}\r",
            editor.screen_rows, editor.screen_cols
        );
        editor.read_key();
        editor
    }

    fn iscntrl(&self, c: u8) -> bool {
        c <= 31
    }

    // #[allow(non_snake_case)]
    // // returns the asci value of ctrl+{c}
    // fn CTRL_KEY(c: char) -> u8 {
    //     c as u8 & 0x1f
    // }

    fn exit(&self) {
        self.clear_screen();
        self.disable_raw_mode();
        std::process::exit(0);
    }

    // *** Terminal ***
    fn disable_raw_mode(&self) {
        termios::tcsetattr(self.stdin_fileno, termios::TCSAFLUSH, &self.orig_termios)
            .expect("Error reverting terminal to original state");
    }

    fn enable_raw_mode(&self) {
        // get and current terminal flags
        use termios::*;
        let mut raw = self.orig_termios.clone();
        tcgetattr(self.stdin_fileno, &mut raw).expect("Error getting terminal attrs");

        // Configure flags for raw mode
        raw.c_iflag &= !(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
        raw.c_oflag &= !(OPOST);
        raw.c_cflag |= CS8;
        raw.c_lflag &= !(ECHO | ICANON | IEXTEN | ISIG);
        // Set timeout on reads
        raw.c_cc[VMIN] = 0;
        raw.c_cc[VTIME] = 1;

        // Set flags and return
        tcsetattr(self.stdin_fileno, TCSAFLUSH, &raw).expect("Error setting terminal to raw mode");
    }

    fn read_key(&self) -> u8 {
        let mut next = [0; 1];
        match stdin().read_exact(&mut next) {
            Ok(_) => next[0],
            Err(e) => match e.kind() {
                // If our read timed out, set c to zero
                ErrorKind::UnexpectedEof => 0,
                _ => panic!(e),
            },
        }
    }

    fn get_window_size(&mut self) {
        let mut ws = libc::winsize {
            ws_col: 1,
            ws_row: 0,
            ws_ypixel: 0,
            ws_xpixel: 0,
        };
        unsafe {
            if true || ioctl(self.stdout_fileno, libc::TIOCGWINSZ, &mut ws) == 0 || ws.ws_col == 0 {
                if stdout().write(b"\x1b[999C\x1b[999B").unwrap() != 12 {
                    panic!("Unable to get screen size with fallback method");
                } else {
                    self.get_cursor_position();
                    self.read_key();
                    return;
                }
            } else {
                self.screen_rows = ws.ws_row;
                self.screen_cols = ws.ws_col;
            }
        }
    }

    fn get_cursor_position(&mut self) {
        let mut buf = [0; 8];
        let mut should_break = true;
        if stdout().write(b"\x1b[6n").unwrap() != 4 {
            panic!("Failed at get cursor position in fallback method")
        }
        // while !should_break {
        //     stdin().read(&mut buf).unwrap();
        //     // for c in buf.iter() {
        //     //     if c == &('R' as u8) {
        //     //         should_break = true;
        //     //         break;
        //     //     }
        //     // }
        // }
        // println!("{:?}\r", buf);
        self.read_key();

        // self.screen_rows = buf[1] as u16;
        // self.screen_cols = buf[0] as u16;
    }

    // *** INPUT ***
    fn process_keypress(&self) {
        let c = self.read_key();
        match c {
            // CTRL_KEY('q')
            17 => self.exit(),
            _ => println!("{}\r", c),
        }
    }

    fn run(&self) {
        self.process_keypress();
        // loop {
        //     self.refresh_screen();
        //     self.process_keypress();
        // }
        // disable_raw_mode(ORIG_TERMIOS);
    }

    // *** OUTPUT ***
    fn refresh_screen(&self) {
        self.clear_screen();
        self.draw_rows();
        // Move cursor back to top left
        stdout().write(b"\x1b[H").unwrap();
    }
    fn draw_rows(&self) {
        for _ in 0..=self.screen_rows {
            stdout().write(b"~\r\n").unwrap();
        }
    }
    fn clear_screen(&self) {
        // Clear screen
        stdout().write(b"\x1b[2J").unwrap();
        // Move cursor to top left
        stdout().write(b"\x1b[H").unwrap();
    }
}
// *** INIT ***
fn main() {
    let editor = Editor::new();
    editor.enable_raw_mode();
    match catch_unwind(|| editor.run()) {
        Ok(_) => (),
        Err(e) => {
            editor.clear_screen();
            editor.disable_raw_mode();
            panic!(e)
        }
    }
    editor.disable_raw_mode();
}
