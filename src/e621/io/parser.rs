/// Parser that reads a tag file and parses the tags.
pub struct Parser {
    /// Current cursor position in the array of characters.
    pub pos: usize,
    /// Input used for parsing.
    pub input: String,
}

pub trait ParserFnc {
    /// Get reference of `Parser`.
    fn borrow(&self) -> &Parser;

    /// Get mutable reference of `Parser`.
    fn borrow_mut(&mut self) -> &mut Parser;

    /// Consume and discard zero or more whitespace characters.
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// Consumes characters until `test` returns false.
    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while !self.eof() && test(self.next_char()) {
            result.push(self.consume_char());
        }

        result
    }

    /// Returns current char and pushes `self.pos` to the next char.
    fn consume_char(&mut self) -> char {
        let mut iter = self.get_current_input().char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.borrow_mut().pos += next_pos;
        cur_char
    }

    /// Read the current char without consuming it.
    fn next_char(&mut self) -> char {
        self.get_current_input().chars().next().unwrap()
    }

    /// Checks if the current input starts with the given string.
    fn starts_with(&self, s: &str) -> bool {
        self.get_current_input().starts_with(s)
    }

    /// Gets current input from current `pos` onward.
    fn get_current_input(&self) -> &str {
        &self.borrow().input[self.borrow().pos..]
    }

    /// Checks whether or not `pos` is at end of file.
    fn eof(&self) -> bool {
        self.borrow().pos >= self.borrow().input.len()
    }
}

impl ParserFnc for Parser {
    fn borrow(&self) -> &Parser {
        self
    }

    fn borrow_mut(&mut self) -> &mut Parser {
        self
    }
}
