use std::io::{stdin, stdout, ErrorKind, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
// use std::panic::catch_unwind;

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
    // Cursor x, cursor y
    cx: usize,
    cy: usize,
    screen_rows: usize,
    screen_cols: usize,
    // version: &'static str,
}

impl Drop for Editor {
    fn drop(&mut self) {
        self.clear_screen();
        self.disable_raw_mode();
    }
}

impl Editor {
    pub fn new() -> Editor {
        let mut editor = Editor {
            orig_termios: Termios::from_fd(stdin().as_raw_fd()).unwrap(),
            stdin_fileno: stdin().as_raw_fd(),
            stdout_fileno: stdout().as_raw_fd(),
            screen_rows: 0,
            screen_cols: 0,
            cx: 1,
            cy: 0,
            // version: "0.0.1",
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

    // fn iscntrl(&self, c: u8) -> bool {
    //     c <= 31
    // }

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
        // Buffer for next character
        let mut next = [0; 1];
        // let mut seq = [0; 3];
        match stdin().read_exact(&mut next) {
            Ok(_) => {
                match next[0] {
                    27 => {
                        // Buffer for escape sequence.
                        let mut seq = [0; 3];
                        match stdin().read(&mut seq) {
                            Ok(_) => {
                                // if we get '[' at position zero it's a generated response
                                if seq[0] == 91 {
                                    match seq[1] {
                                        // A => w
                                        65 => 119,
                                        // B => s
                                        66 => 115,
                                        // C => d
                                        67 => 100,
                                        // D => a
                                        68 => 97,
                                        0 => next[0],
                                        _ => next[0],
                                    }
                                } else {
                                    next[0]
                                }
                            }
                            Err(e) => match e.kind() {
                                // If our read timed out, set c to zero
                                ErrorKind::UnexpectedEof => 0,
                                _ => panic!(e),
                            },
                        }
                    }
                    _ => next[0],
                }
            }
            Err(e) => match e.kind() {
                // If our read timed out, set c to zero
                ErrorKind::UnexpectedEof => 0,
                _ => panic!(e),
            },
        }
    }
    // match next[1] {
    //                         // A => w
    //                         65 => 119,
    //                         // B => s
    //                         66 => 115,
    //                         // C => d
    //                         67 => 100,
    //                         // D => a
    //                         68 => 97,
    //                         0 => next[0],
    //                         _ => next[0]

    //                     },
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
                    panic!("Unable to move to bottom right corner");
                } else {
                    // Use the cursor's location to tell the size of the window
                    self.get_cursor_position();
                    return;
                }
            } else {
                self.screen_rows = ws.ws_row as usize;
                self.screen_cols = ws.ws_col as usize;
            }
        }
    }

    fn get_cursor_position(&mut self) {
        let mut buf = [0; 32];
        let mut rows: usize = 0;
        let mut cols: usize = 0;
        let mut index: usize = 0;
        // Send the command to get cursor position. We will be able to read the response at stdin
        if stdout().write(b"\x1b[6n").unwrap() != 4 {
            panic!("Failed at get cursor position in fallback method")
        }
        // Force flush so buffering doesn't delay our command
        stdout().flush().unwrap();
        // Read the value returned by the terminal
        stdin().read(&mut buf).unwrap();
        // The buffer now contains "\x1b[" (chars 71, 91) at some index
        // we want to find that index. The full response is "\x1b{rows};{cols}R"
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
            rows = rows + (buf[index] - 48) as usize;
            index = index + 1;
        }
        // After the semicolon (char 59) is the col number followed by 'R' (char 82)
        index = index + 1;
        while buf[index] != 82 {
            // Convert the ascii col number to an integer
            cols = cols * 10;
            cols = cols + (buf[index] - 48) as usize;
            index = index + 1;
        }
        self.screen_rows = rows;
        self.screen_cols = cols;
    }

    // *** INPUT ***
    fn process_keypress(&mut self) {
        let c = self.read_key();
        match c {
            // Nothing - do nothing
            0 => (),
            // w, a s, d
            97 | 100 | 115 | 119 => self.move_cursor(c),
            // CTRL_KEY('q')
            17 => self.exit(),
            107 => self.refresh_screen(),
            _ => println!("{}\r", c),
        }
    }

    fn move_cursor(&mut self, c: u8) {
        match c as char {
            'w' => {
                if self.cy > 0 {
                    self.cy = self.cy - 1
                }
            }
            's' => {
                if self.cy < self.screen_rows {
                    self.cy = self.cy + 1
                }
            }
            'a' => {
                if self.cx > 1 {
                    self.cx = self.cx - 1
                }
            }
            'd' => {
                if self.cx < self.screen_cols {
                    self.cx = self.cx + 1
                }
            }
            _ => (),
        }
    }

    fn run(&mut self) {
        // self.refresh_screen();
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
        let cursor_position = format!("\x1b[{0};{1}H", self.cy + 1, self.cx + 1);
        // Move cursor to top left and show
        output.push_str(&cursor_position);
        output.push_str("\x1b[?25h");
        // Write all commands to stdout at once
        stdout().write(output.as_bytes()).unwrap();
        stdout().flush().unwrap();
    }
    fn draw_rows(&self, output: &mut String) {
        let welcome_msg = concat!(
            "ViMacs Editor -- Version ",
            env!("CARGO_PKG_VERSION"),
            "\r\n"
        );
        for i in 0..self.screen_rows - 1 {
            if i == self.screen_rows / 3 {
                if welcome_msg.len() > self.screen_cols {
                    output.push_str(&welcome_msg[0..self.screen_cols]);
                } else {
                    let padding = (self.screen_cols - welcome_msg.len()) / 2;
                    if padding > 0 {
                        output.push_str("~");
                    }
                    for _ in 0..padding - 1 {
                        output.push_str(" ");
                    }
                    output.push_str(&welcome_msg);
                }
            } else {
                // Write a tilde, clear the rest of the line, then return and newline
                output.push_str("~\x1b[K\r\n");
            }
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
    let mut editor = Editor::new();
    editor.run()
}
