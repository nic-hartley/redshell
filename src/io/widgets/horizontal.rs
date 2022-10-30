use crate::io::output::{Cell, Screen};

pub struct Horizontal<'a> {
    pub(in super::super) screen: &'a mut Screen,
    pub(in super::super) row: usize,
    pub(in super::super) start: Option<usize>,
    pub(in super::super) end: Option<usize>,
    pub(in super::super) char: char,
}

impl<'a> Horizontal<'a> {
    pub fn new(screen: &'a mut Screen, row: usize) -> Self {
        Horizontal {
            screen,
            row,
            start: None,
            end: None,
            char: '-',
        }
    }

    crate::util::setters! {
        start(x: usize) => start = Some(x),
        end(x: usize) => end = Some(x),
        char(ch: char) => char = ch,
    }
}

crate::util::abbrev_debug! {
    Horizontal<'a>;
    write row,
    if start != None,
    if end != None,
    if char != '-',
}

impl<'a> Drop for Horizontal<'a> {
    fn drop(&mut self) {
        let start_x = self.start.unwrap_or(0);
        let end_x = self.end.unwrap_or(self.screen.size().x());
        for x in start_x..end_x {
            self.screen[self.row][x] = Cell::plain(self.char);
        }
    }
}