/// The `BaseParser` is responsible for parsing files character-by-character without any inherit rule.
/// This is a thin blanket for other parsers to use and build rules, allowing for quick and easy
/// parsing for any file.
///
/// # Note:
/// This doesn't use advanced libraries for lexing and/or data storage. Serde, and any other library
/// that generally tries to perform those tasks aren't useful for this class, which is meant to collect
/// data in a minimalist, none-structural, none-grammatical way. This class has no use for large libraries meant for
/// more typical syntax.
#[derive(Default)]
pub struct BaseParser {
    /// Current cursor position in the array of characters.
    pub pos: usize,
    /// Input used for parsing.
    pub input: String,
}

impl BaseParser {
    /// Consume and discard zero or more whitespace characters.
    pub fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// Consumes characters until `test` returns false.
    pub fn consume_while<F>(&mut self, test: F) -> String
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
    pub fn consume_char(&mut self) -> char {
        let mut iter = self.get_current_input().char_indices();
        let (_, cur_char) = iter.next().unwrap();
        let (next_pos, _) = iter.next().unwrap_or((1, ' '));
        self.pos += next_pos;
        cur_char
    }

    /// Read the current char without consuming it.
    pub fn next_char(&mut self) -> char {
        self.get_current_input().chars().next().unwrap()
    }

    /// Checks if the current input starts with the given string.
    pub fn starts_with(&self, s: &str) -> bool {
        self.get_current_input().starts_with(s)
    }

    /// Gets current input from current `pos` onward.
    pub fn get_current_input(&self) -> &str {
        &self.input[self.pos..]
    }

    /// Checks whether or not `pos` is at end of file.
    pub fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }
}
