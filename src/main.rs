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
        editor.enable_raw_mode();
        editor.clear_screen();
        editor.get_window_size();
        println!(
            "Rows: {0}, Cols; {1}",
            editor.screen_rows, editor.screen_cols
        );
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
        // raw.c_cc[VMIN] = 0;
        // raw.c_cc[VTIME] = 1;

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
            ws_col: 0,
            ws_row: 0,
            ws_ypixel: 0,
            ws_xpixel: 0,
        };
        unsafe {
            // try using libc's ioctl tp get the terminal size
            if ioctl(self.stdout_fileno, libc::TIOCGWINSZ, &mut ws) == 0 || ws.ws_col == 0 {
                // if that fails we need to do it manually so...
                // Move the cursor to bottom right corner of the screen
                if stdout().write(b"\x1b[999C\x1b[999B").unwrap() != 12 {
                    panic!("Unable to get screen size with fallback method");
                } else {
                    // Use the cursor's location to tell the size of the window
                    self.get_cursor_position();
                    return;
                }
            } else {
                self.screen_rows = ws.ws_row;
                self.screen_cols = ws.ws_col;
            }
        }
    }

    fn get_cursor_position(&mut self) {
        let mut buf = [0; 32];
        let mut rows: u16 = 0;
        let mut cols: u16 = 0;
        let mut index: usize = 0;
        // Send the command to get cursor position. We will be able to read the response at stdin
        if stdout().write(b"\x1b[6n").unwrap() != 4 {
            panic!("Failed at get cursor position in fallback method")
        }
        // Force flush so buffering doesn't throw off the timing of our read
        stdout().flush().unwrap();
        // Read the value returned by the terminal
        stdin().read(&mut buf).unwrap();
        // The buffer now contains "\x1b[" (chars 71, 91) at some index
        // we want to find that index
        for i in 0..buf.len() {
            if buf[i] == 27 && buf[i + 1] == 91 {
                index = i;
                break;
            }
        }
        if buf[index] != 27 || buf[index + 1] != 91 {
            panic!("Did not read cursor position!");
        }
        // After "\x1b[" is the row number followed by a semicolon (char 59)
        index = index + 2;
        while buf[index] != 59 {
            // Convert the ascii row number to an integer
            rows = rows * 10;
            rows = rows + (buf[index] - 48) as u16;
            index = index + 1;
        }
        // After the semicolon (char 59) is the col number followed by 'R' (char 82)
        index = index + 1;
        while buf[index] != 82 {
            // Convert each the ascii col number to an integer
            cols = cols * 10;
            cols = cols + (buf[index] - 48) as u16;
            index = index + 1;
        }
        self.screen_rows = rows;
        self.screen_cols = cols;
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
        loop {
            self.refresh_screen();
            self.process_keypress();
        }
    }

    // *** OUTPUT ***
    fn refresh_screen(&self) {
        let mut output = String::new();
        // Hide cursor, Move to top left
        output.push_str("\x1b[?25l\x1b[H");
        // Clear rows and push new contents to output string
        self.draw_rows(&mut output);
        // Move cursor to top left and show
        output.push_str("\x1b[H\x1b[?25h");
        // Write all commands to stdout at once
        stdout().write(output.as_bytes()).unwrap();
    }
    fn draw_rows(&self, output: &mut String) {
        for _ in 0..self.screen_rows - 1 {
            // Write a tilde, clear the rest of the line, then return and newline
            output.push_str("~\x1b[K\r\n");
        }
        output.push_str("~\x1b[K");
    }
    fn clear_screen(&self) {
        // Clear screen, move cursor to top left
        stdout().write(b"\x1b[2J\x1b[H").unwrap();
    }
}
// *** INIT ***
fn main() {
    let editor = Editor::new();
    match catch_unwind(|| editor.run()) {
        Ok(_) => (),
        Err(e) => {
            editor.clear_screen();
            editor.disable_raw_mode();
            panic!(e)
        }
    }
    editor.disable_raw_mode()
}
