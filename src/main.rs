use std::env;
mod editor;
mod row;
use editor::Editor;


// *** INIT ***
fn main() {
    let mut editor = Editor::new();
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        editor.open(&args[1]);
    }
    editor.run()
}
