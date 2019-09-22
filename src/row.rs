use crate::editor::{Highlight, TAB_STOP};
pub struct Row {
    text: String,
    rendered: String,
    highlight: Vec<Highlight>,
}
fn is_separator(c: char) -> bool {
    for ch in " \0,.()+-/*=~%<>[];".chars() {
        if c == ch {
            return true;
        }
    }
    false
}

impl Row {
    pub fn new() -> Row {
        Row {
            text: String::new(),
            rendered: String::new(),
            highlight: Vec::new(),
        }
    }

    pub fn from(text: String) -> Row {
        let mut row = Row {
            text,
            rendered: String::new(),
            highlight: Vec::new(),
        };
        row.render();
        row.update_highlight();
        row
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }
    pub fn get(&self, low: usize, high: usize) -> &str {
        if low >= self.len() {
            &""
        } else if high >= self.len() {
            &self.text[low..self.len()]
        } else {
            &self.text[low..high]
        }
    }
    pub fn get_render_slice(&self, low: usize, high: usize) -> &str {
        if low >= self.len() {
            &""
        } else if high >= self.len() {
            &self.rendered[low..self.len()]
        } else {
            &self.rendered[low..high]
        }
    }
    pub fn get_render(&self) -> &str {
        &self.rendered
    }
    pub fn search(&self, query: &str) -> Option<usize> {
        return self.text.find(query);
    }
    //search string after given index
    pub fn search_from(&self, index: usize, query: &str) -> Option<usize> {
        if let Some(res_index) = self.get(index + 1, self.len()).find(query) {
            Some(index + res_index + 1)
        } else {
            None
        }
    }
    // pub fn search_to(&self, index: usize, query: &str) -> Option<usize> {
    //     return self.get(0, index + query.len()).find(query);
    // }
    pub fn search_reverse(&self, query: &str) -> Option<usize> {
        if self.len() < query.len() {
            return None;
        };
        for i in (0..=(self.len() - query.len())).rev() {
            if self.get(i, i + query.len()) == query {
                return Some(i);
            }
        }
        None
    }
    // Search row for query up to but not including index as a starting position for string
    pub fn search_reverse_to(&self, index: usize, query: &str) -> Option<usize> {
        for i in (0..index).rev() {
            if self.get(i, i + query.len()) == query {
                return Some(i);
            }
        }
        None
    }
    pub fn split_off(&mut self, index: usize) -> Row {
        let next_text = self.text.split_off(index);
        self.render();
        Row::from(next_text)
    }
    pub fn render(&mut self) {
        let mut rendered = String::new();
        for c in self.text.chars() {
            if c == '\t' {
                for _ in 0..TAB_STOP {
                    rendered.push(' ');
                }
            } else {
                rendered.push(c);
            }
        }
        self.rendered = rendered;
    }
    pub fn insert(&mut self, index: usize, c: char) {
        self.text.insert(index, c);
        self.render();
        self.update_highlight();
    }
    pub fn join(&mut self, new: &Row) {
        self.text.push_str(&new.text);
        self.rendered.push_str(&new.rendered);
    }
    pub fn remove(&mut self, index: usize) {
        self.text.remove(index);
        self.render();
    }
    pub fn cx_to_rx(&self, cx: usize) -> usize {
        let mut rx: usize = 0;
        let end = std::cmp::min(cx, self.len());
        for c in self.text[..end].chars() {
            if c == '\t' {
                rx += (TAB_STOP - 1) - (rx % TAB_STOP);
            }
            rx += 1;
        }
        rx
    }
    pub fn update_highlight(&mut self) {
        let mut prev_was_separator = true;
        let mut prev_hl = Highlight::Normal;
        if self.highlight.len() < self.rendered.len() {
            self.highlight
                .resize(self.rendered.len(), Highlight::Normal);
        }
        for (i, c) in self.rendered.chars().enumerate() {
            if c.is_numeric() && (prev_was_separator || prev_hl == Highlight::Number) {
                self.highlight[i] = Highlight::Number;
            } else {
                self.highlight[i] = Highlight::Normal;
            }
            prev_hl = self.highlight[i].clone();
            prev_was_separator = is_separator(c);
        }
    }
    pub fn get_text(&self) -> &str {
        &self.text
    }
    pub fn get_highlight_at(&self, index: usize) -> &Highlight {
        if index >= self.highlight.len() {
            return &Highlight::Normal;
        }
        &self.highlight[index]
    }
    pub fn set_highlight_from(&mut self, highlight: Highlight, start: usize, distance: usize) {
        for i in start..(start + distance) {
            if i >= self.highlight.len() {
                return;
            };
            self.highlight[i] = highlight.clone();
        }
    }
    pub fn save_highlight(&self, start: usize, distance: usize) -> Option<Vec<Highlight>> {
        if start >= self.highlight.len() {
            return None;
        }
        let mut saved = vec![];
        for i in start..(start + distance) {
            if i >= self.highlight.len() {
                break;
            }
            saved.push(self.highlight[i].clone());
        }
        Some(saved)
    }
    pub fn set_highlight_group(&mut self, start: usize, highlights: &Vec<Highlight>) {
        if start >= self.highlight.len() {
            return;
        }
        for i in start..start + highlights.len() {
            if i >= self.highlight.len() {
                return;
            }
            self.highlight[i] = highlights[i - start].clone();
        }
    }
}
