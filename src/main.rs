use std::env;
use std::fs::File;
use std::io::{stdin, stdout, BufRead, BufReader, ErrorKind, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
// use std::panic::catch_unwind;

extern crate termios;
use termios::Termios;

// extern crate libc;
// use libc::ioctl;
// extern crate nix;
// use nix::sys::ioctl;

// *** Defines ***
const ARROW_UP: u16 = 1000;
const ARROW_LEFT: u16 = 1001;
const ARROW_RIGHT: u16 = 1002;
const ARROW_DOWN: u16 = 1003;
const PAGE_UP: u16 = 1004;
const PAGE_DOWN: u16 = 1005;
const HOME_KEY: u16 = 1006;
const END_KEY: u16 = 1007;
const DELETE_KEY: u16 = 1008;
const LEFT_BRACKET: u8 = 91;
const ESCAPE: u8 = 27;

struct Editor {
    orig_termios: Termios,
    stdin_fileno: RawFd,
    // stdout_fileno: RawFd,
    // Cursor x, cursor y
    cx: usize,
    cy: usize,
    row_offset: usize,
    col_offset: usize,
    screen_rows: usize,
    screen_cols: usize,
    tab_stop: usize,
    rx: usize,
    rows: Vec<String>, // version: &'static str
}

impl Drop for Editor {
    fn drop(&mut self) {
        // self.clear_screen();
        self.disable_raw_mode();
    }
}

impl Editor {
    // use crate::EditorKey;
    pub fn new() -> Editor {
        let mut editor = Editor {
            orig_termios: Termios::from_fd(stdin().as_raw_fd()).unwrap(),
            stdin_fileno: stdin().as_raw_fd(),
            // stdout_fileno: stdout().as_raw_fd(),
            screen_rows: 0,
            screen_cols: 0,
            cx: 1,
            cy: 0,
            row_offset: 0,
            col_offset: 0,
            rows: Vec::new(),
            tab_stop: 4,
            rx: 0,
            // version: "0.0.1",
        };
        editor.enable_raw_mode();
        editor.clear_screen();
        editor.get_window_size();
        editor
    }

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

    fn read_key(&self) -> u16 {
        // Buffer for next character
        let mut next = [0; 1];
        // let mut seq = [0; 3];
        match stdin().read_exact(&mut next) {
            Ok(_) => {
                match next[0] {
                    ESCAPE => {
                        // Buffer for escape sequence.
                        let mut seq = [0; 3];
                        match stdin().read(&mut seq) {
                            Ok(_) => {
                                // if we get '[' at position zero it's a generated response
                                if seq[0] == LEFT_BRACKET {
                                    if seq[1] >= '0' as u8 && seq[1] <= '9' as u8 {
                                        if seq[2] == '~' as u8
                                        //|| seq[2] == 'C' as u8
                                        {
                                            match seq[1] {
                                                1 => HOME_KEY,
                                                3 => DELETE_KEY,
                                                4 => END_KEY,
                                                5 => PAGE_UP,
                                                6 => PAGE_DOWN,
                                                7 => HOME_KEY,
                                                8 => END_KEY,
                                                53 => PAGE_UP,
                                                54 => PAGE_DOWN,
                                                _ => ESCAPE as u16,
                                            }
                                        } else {
                                            ESCAPE as u16
                                        }
                                    } else {
                                        match seq[1] {
                                            53 => PAGE_UP,
                                            54 => PAGE_DOWN,
                                            // A
                                            65 => ARROW_UP,
                                            // B
                                            66 => ARROW_DOWN,
                                            // C
                                            67 => ARROW_RIGHT,
                                            // D
                                            68 => ARROW_LEFT,
                                            // H
                                            72 => HOME_KEY,
                                            // F
                                            70 => END_KEY,
                                            // H
                                            _ => ESCAPE as u16,
                                        }
                                    }
                                } else if seq[0] == 'O' as u8 {
                                    match seq[1] {
                                        // H
                                        72 => HOME_KEY,
                                        // F
                                        70 => END_KEY,
                                        _ => ESCAPE as u16,
                                    }
                                } else if seq[0] == 'C' as u8 {
                                    match seq[1] {
                                        // A
                                        65 => PAGE_UP,
                                        // B
                                        66 => PAGE_DOWN,
                                        // C
                                        67 => END_KEY,
                                        // D
                                        68 => HOME_KEY,
                                        _ => ESCAPE as u16,
                                    }
                                } else {
                                    ESCAPE as u16
                                }
                            }
                            Err(e) => match e.kind() {
                                // If our read timed out, set c to zero
                                ErrorKind::UnexpectedEof => 0,
                                _ => panic!(e),
                            },
                        }
                    }
                    _ => next[0] as u16,
                }
            }
            Err(e) => match e.kind() {
                // If our read timed out, set c to zero
                ErrorKind::UnexpectedEof => 0,
                _ => panic!(e),
            },
        }
    }
    fn get_window_size(&mut self) {
        // Move the cursor to bottom right corner of the screen
        if stdout().write(b"\x1b[999C\x1b[999B").unwrap() != 12 {
            panic!("Unable to move to bottom right corner");
        } else {
            // Use the cursor's location to tell the size of the window
            self.get_cursor_position();
            return;
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
            if buf[i] == ESCAPE && buf[i + 1] == LEFT_BRACKET {
                index = i;
                break;
            }
        }
        if buf[index] != ESCAPE || buf[index + 1] != LEFT_BRACKET {
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

    // *** FILE I/O ***
    fn open(&mut self, file_name: &str) {
        let file = File::open(file_name).expect(&format!("Could not open {0}", file_name));
        let buf_reader = BufReader::new(file);
        for line in buf_reader.lines().map(|l| l.unwrap()) {
            self.rows.push(line);
        }
    }

    // *** INPUT ***
    fn process_keypress(&mut self) {
        let c = self.read_key();
        match c {
            // Nothing - do nothing
            0 => (),
            ARROW_UP | ARROW_DOWN | ARROW_LEFT | ARROW_RIGHT => self.move_cursor(c),
            PAGE_DOWN => {
                self.cy = self.row_offset + self.screen_rows - 1;
                for _ in 0..self.screen_rows - 1 {
                    self.move_cursor(ARROW_DOWN)
                }
            }
            PAGE_UP => {
                self.cy = self.row_offset;
                for _ in 0..self.screen_rows - 1 {
                    self.move_cursor(ARROW_UP)
                }
            }
            HOME_KEY => {
                self.cx = 0;
                // for _ in 0..self.screen_cols - 1 {
                //     self.move_cursor(ARROW_LEFT)
                // }
            }
            END_KEY => {
                self.cx = self.screen_cols - 1;
                // for _ in 0..self.screen_cols - 1 {
                //     self.move_cursor(ARROW_RIGHT)
                // }
            }
            // CTRL_KEY('q')
            17 => self.exit(),
            107 => self.refresh_screen(),
            _ => println!("{}\r", c),
        }
    }

    fn move_cursor(&mut self, c: u16) {
        let row_exists = self.cy < self.rows.len();
        let row_size = if row_exists {
            self.rows[self.cy].len()
        } else {
            0
        };
        match c {
            ARROW_UP => {
                if self.cy != 0 {
                    self.cy -= 1
                }
            }
            ARROW_DOWN => {
                if self.cy < self.rows.len() {
                    self.cy += 1
                }
            }
            ARROW_LEFT => {
                if self.cx != 0 {
                    self.cx -= 1;
                } else if self.cy > 0 {
                    self.cy -= 1;
                    self.cx = self.rows[self.cy].len();
                }
            }
            ARROW_RIGHT => {
                if self.cx < row_size {
                    self.cx += 1
                } else if row_exists && self.cx == row_size {
                    self.cy += 1;
                    self.cx = 0;
                }
            }
            _ => (),
        }
        let new_row_exists = self.cy < self.rows.len();
        let new_row_len = if new_row_exists {
            self.rows[self.cy].len()
        } else {
            0
        };
        if self.cx > new_row_len {
            self.cx = new_row_len;
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
    fn refresh_screen(&mut self) {
        self.scroll();
        let mut output = String::new();
        // Hide cursor, Move to top left
        output.push_str("\x1b[?25l\x1b[H");
        // Clear rows and push new contents to output string
        self.draw_rows(&mut output);
        let cursor_position = format!(
            "\x1b[{0};{1}H",
            (self.cy - self.row_offset) + 1,
            (self.rx - self.col_offset) + 1
        );
        // Move cursor to top left and show
        output.push_str(&cursor_position);
        output.push_str("\x1b[?25h");
        // Write all commands to stdout at once
        stdout().write(output.as_bytes()).unwrap();
        stdout().flush().unwrap();
    }
    fn scroll(&mut self) {
        self.rx = 0;
        if self.cy < self.rows.len() {
            self.rx = self.cx_to_rx(&self.rows[self.cy])
        }
        if self.cy < self.row_offset {
            self.row_offset = self.cy;
        }
        if self.cy >= self.row_offset + self.screen_rows {
            self.row_offset = self.cy - self.screen_rows + 1;
        }
        if self.rx < self.col_offset {
            self.col_offset = self.rx;
        }
        if self.rx >= self.col_offset + self.screen_cols {
            self.col_offset = self.rx - self.screen_cols + 1
        }
    }
    fn cx_to_rx(&self, row: &str) -> usize {
        let mut rx: usize = 0;
        let end = std::cmp::min(self.cx, row.len());
        for c in row[..end].chars() {
            if c == '\t' {
                rx += (self.tab_stop - 1) - (rx % self.tab_stop);
            }
            rx += 1;
        }
        rx
        // if rx > 0 {
        //     return rx;
        // }
        // 1
    }
    fn render_string(&self, target: &str) -> String {
        let mut rendered = String::new();
        for c in target.chars() {
            if c == '\t' {
                for _ in 0..self.tab_stop {
                    rendered.push(' ');
                }
            } else {
                rendered.push(c);
            }
        }
        rendered
    }
    fn draw_rows(&self, output: &mut String) {
        let welcome_msg = concat!("ViMacs Editor -- Version ", env!("CARGO_PKG_VERSION"));
        for i in 0..self.screen_rows {
            let current_row = i + self.row_offset;
            if current_row >= self.rows.len() {
                if self.rows.len() == 0 && i == self.screen_rows / 4 {
                    if welcome_msg.len() > self.screen_cols {
                        output.push_str(&welcome_msg[0..self.screen_cols]);
                    } else {
                        let padding = (self.screen_cols - welcome_msg.len()) / 2;
                        if padding > 0 {
                            output.push('~');
                        }
                        for _ in 0..padding - 1 {
                            output.push(' ');
                        }
                        output.push_str(&welcome_msg);
                    }
                } else {
                    // Write a tilde, clear the rest of the line, then return and newline
                    output.push('~');
                }
            } else {
                output.push('~');
                let rendered_row = self.render_string(&self.rows[current_row]);
                let end = std::cmp::min(self.screen_cols + self.col_offset - 1, rendered_row.len());
                if self.col_offset < rendered_row.len() {
                    output.push_str(&rendered_row[self.col_offset..end]);
                }
            }
            if i == self.screen_rows - 1 {
                output.push_str("\x1b[K");
            } else {
                output.push_str("\x1b[K\r\n");
            }
        }
    }
    fn clear_screen(&self) {
        // Clear screen, move cursor to top left
        stdout().write(b"\x1b[2J\x1b[H").unwrap();
    }
}
// *** INIT ***
fn main() {
    let mut editor = Editor::new();
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        editor.open(&args[1]);
    }
    editor.run()
}
