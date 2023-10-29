// A Rust pretty-printer implementation based on the OCaml stdlib Formatter
//
// TODO@@
//
// Write tests for formatter
//
// Define PP with method fprintf/pprint which takes:

use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    ops::Add,
};

// pub trait Pretty {
//     fn prettify(&self, ppf: &mut BufPrinter<'_>) -> String;
// }

/*
* Handling usize -> i32 conversion.
*/
trait LenAsI32 {
    fn len_i32(&self) -> i32;
}

trait SizeAsI32
where
    Self: LenAsI32,
{
    fn size_i32(&self) -> i32;
}

trait CountAsI32 {
    fn count_i32(&self) -> i32;
}

impl LenAsI32 for str {
    fn len_i32(&self) -> i32 {
        self.len().try_into().unwrap()
    }
}

const INFINITY: i32 = 1000000010;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
struct Size(i32);

impl Size {
    // Zero size
    pub const ZERO: Size = Size(0);

    // Unknown size
    pub const UNKNOWN: Size = Size(-1);

    // pub const INFINITY: Size = Size(1000000010);

    // Check if the size is known
    pub fn is_known(&self) -> bool {
        self.0 >= 0
    }
}

// Convert from an i32 to Size
impl From<i32> for Size {
    fn from(value: i32) -> Self {
        Size(value)
    }
}
//
// Convert from an i32 to Size
impl From<usize> for Size {
    fn from(value: usize) -> Self {
        // probably be careful of overflows etc
        Size(value as i32)
    }
}

// Convert from Size to i32
impl From<&Size> for i32 {
    fn from(size: &Size) -> i32 {
        size.0
    }
}

impl Add<i32> for Size {
    type Output = Size;

    fn add(self, rhs: i32) -> Self::Output {
        Size(self.0 + rhs)
    }
}

/* The pretty-printing boxes definition:
* - hbox: a horizontal box (no line splitting)
* - vbox: vertical box (every break hint splits the line)
* - hvbox: horizontal/veritcal box
*   (the box behaves as a hortizontal box if it fits on
*   the current line, otherwise the box behaves as a vertical box)
* - hovbox: horizontal or vertical compacting box
*   (the box is compacting material, printing as much as possible
*   on every line)
* - box: horizontal or vertical compacting box with enhanced box structure
*   (the box behaves as an horizontal or vertical box but break hints split
*      the line if splitting would move to the left)
*/
#[derive(Clone, Debug)]
pub enum PpBox {
    Hbox(),
    Vbox(),
    Hvbox(),
    Hovbox(),
    Box(),
    Fits(),
}

#[derive(Clone, Debug)]
enum PpToken {
    Text(String), // normal text
    Break {
        fits: (String, i32, String),   // line is not split
        breaks: (String, i32, String), // line is split
    },
    Begin(i32, PpBox), // Beginning of a box
    End(),             // End of a box
    Newline(),         // Force a newline inside a box
                       // OpenTag(), // opening a tag name
                       // CloseTag(), // closing the most recently opened tab
}

impl LenAsI32 for PpToken {
    fn len_i32(&self) -> i32 {
        match self {
            PpToken::Text(s) => s.len_i32(),
            PpToken::Break { fits, breaks } => 0 as i32,
            PpToken::Begin(_, _) => 0 as i32,
            PpToken::End() => 0 as i32,
            PpToken::Newline() => 0 as i32,
        }
    }
}

impl PpToken {
    fn len(&self) -> usize {
        match self {
            PpToken::Text(s) => s.len(),
            PpToken::Break { fits, breaks } => 0,
            PpToken::Begin(_, _) => 0,
            PpToken::End() => 0,
            PpToken::Newline() => 0,
        }
    }
}

impl SizeAsI32 for PpToken {
    fn size_i32(&self) -> i32 {
        match self {
            PpToken::Text(s) => s.len_i32(),
            PpToken::Break { fits, breaks } => 0,
            PpToken::Begin(_, _) => 0,
            PpToken::End() => 0,
            PpToken::Newline() => 0,
        }
    }
}

// The pretty-printer queue:

// PpQueueT contains a usize which is the token_id reference for self.tokens
#[derive(Clone, Debug)]
pub struct PpQueueT(usize);

/* The pretty-printer scanning stack */

/* The pretty-printer scanning stack: scanning element definition */
#[derive(Debug)]
pub struct PpScanT {
    left_total: i32, // Value of self.left_total when the element was enqueued.
    token_idx: usize,
}

/* The pretty-printer formatting stack:
the formatting stack contains the description of all the currently active
boxes; the pretty-printer formatting stack is used to split the lines
while printing tokens. */

/* The pretty-printer formatting stack: formatting stack element definition.
Each stack element describes a pretty-printing box. */
// PpFormaT is the type of the format stack. usize is the width of the box
// when loading onto the formatting stack. This is never mutated.
#[derive(Debug)]
struct PpFormatT {
    box_type: PpBox,
    box_size: i32,
}
const ELLIPSIS: &str = ".";

#[derive(Debug)]
pub struct BufPrinter {
    // The token mapping.
    tokens: Vec<PpToken>,
    // The token length mapping.
    token_lengths: HashMap<usize, i32>,
    // The token size mapping.
    token_sizes: HashMap<usize, Size>,
    // The pretty-printer scanning stack.
    scan_stack: Vec<PpScanT>,
    // The pretty-printer formatting stack.
    format_stack: Vec<PpFormatT>,
    // The pretty-printer queue.
    queue: VecDeque<PpQueueT>,
    // Value of the right margin.
    margin: i32,
    // Minimum space left before margin, when opening a box.
    min_space_left: i32,
    // Maximum value of indentation: no box can be opened
    // further.
    max_indent: i32,
    // Space remaining on the current line
    space_left: i32,
    // Current value of indentation.
    current_indent: i32,
    // True when the line has been broken by the pretty-printer.
    is_new_line: bool,
    // Total width of tokens already printed.
    left_total: i32,
    // Total width of tokens ever put in the queue.
    right_total: i32,
    // Current number of open boxes.
    curr_depth: i32,
    // Maximum number of boxes which can be open
    max_boxes: i32,
    // Ellipsis string.
    // ellipsis: String,
    // Output function
    // out_string: fn(String, i32, i32) -> (),
    // Flushing function
    // out_flush: fn(),
    // Output of new lines
    // out_newline: fn(),
    // Output of break hints spaces.
    // out_spaces: fn(),
    // The output buffer we write to for now.
    pub out_buf: String,
}

impl BufPrinter {
    pub fn new(
        margin: i32,
        min_space_left: i32,
        max_indent: i32,
        max_boxes: i32,
        // out_string: fn(String, i32, i32) -> (),
        // out_flush: fn(),
        // out_newline: fn(),
        // out_spaces: fn(),
    ) -> Self {
        let tokens: Vec<PpToken> = Vec::new();
        // Token length mapping.
        // let token_lengths: Vec<i32> = Vec::new();
        let token_lengths: HashMap<usize, i32> = HashMap::new();
        // Token size mapping.
        // let token_sizes: Vec<Size> = Vec::new();
        let token_sizes: HashMap<usize, Size> = HashMap::new();
        // The pretty-printer scanning stack.
        let scan_stack: Vec<PpScanT> = Vec::new();
        // The pretty-printer formatting stack.
        let format_stack: Vec<PpFormatT> = Vec::new();
        // The pretty-printer queue.
        let queue: VecDeque<PpQueueT> = VecDeque::new();

        let mut f = BufPrinter {
            tokens,
            token_lengths,
            token_sizes,
            scan_stack,
            format_stack,
            queue,
            margin,
            min_space_left,
            max_indent,
            space_left: margin, // Initially set to the margin value
            current_indent: 0,
            is_new_line: true,
            left_total: 1,
            right_total: 1,
            curr_depth: 1,
            max_boxes,
            // out_string,
            // out_flush,
            // out_newline,
            // out_spaces,
            out_buf: String::new(),
        };
        let sys_tok = PpToken::Begin(0, PpBox::Hovbox());
        let token_idx = f.add_token(sys_tok);
        // f.tokens.push(sys_tok);
        f.token_lengths.insert(token_idx, 0);
        f.token_sizes.insert(token_idx, Size::UNKNOWN);
        f.queue.push_back(PpQueueT(token_idx));

        f.initialise_scan_stack();
        f.scan_stack.push(PpScanT {
            token_idx,
            left_total: 1,
        });
        f
    }

    fn add_token(&mut self, token: PpToken) -> usize {
        self.tokens.push(token);
        return self.tokens.len() - 1;
    }

    fn enqueue(&mut self, token_id: usize) {
        self.right_total += self.token_lengths[&token_id];
        self.queue.push_back(PpQueueT(token_id))
    }

    fn clear_queue(&mut self) {
        self.left_total = 1;
        self.right_total = 1;
        self.queue.clear()
    }

    fn output_string(&mut self, s: &str) {
        self.out_buf.push_str(s);
    }

    pub fn output_newline(&mut self) {
        self.out_buf.push_str("\n");
    }

    fn output_spaces(&mut self, n: i32) {
        for _ in 0..n {
            self.out_buf.push_str(" ");
        }
    }

    pub fn output_indent(&mut self, n: i32) {
        for _ in 0..n {
            self.out_buf.push_str(" ");
        }
    }

    fn format_pp_text(&mut self, s: &str, size: i32) {
        self.space_left -= size;
        self.output_string(s);
        self.is_new_line = false;
    }

    fn format_string(&mut self, s: &str) {
        if s != "" {
            self.format_pp_text(s, s.len_i32())
        }
    }

    // To format a break, indenting a new line.
    fn break_new_line(&mut self, before: &str, offset: i32, after: &str, width: i32) {
        self.format_string(before);
        self.output_newline();
        self.is_new_line = true;
        let indent = self.margin - width + offset;
        // Don't indent more than max_indent.
        let real_indent = min(self.max_indent, indent);
        self.current_indent = real_indent;
        self.space_left = self.margin - self.current_indent;
        self.output_indent(self.current_indent);
        self.format_string(after);
    }

    // To force a line break inside a box: no offset is added.
    fn break_line(&mut self, width: i32) {
        self.break_new_line("", 0, "", width)
    }

    // To format a break that fits on the current line.
    fn break_same_line(&mut self, before: &str, width: i32, after: &str) {
        self.format_string(before);
        self.space_left -= width;
        self.output_spaces(width);
        self.format_string(after)
    }

    fn force_break_line(&mut self) {
        match self.format_stack.last() {
            None => self.output_newline(),
            Some(f) => {
                if f.box_size > self.space_left {
                    match f.box_type {
                        PpBox::Fits() => {}
                        PpBox::Hbox() => {}
                        _ => self.break_line(f.box_size),
                    }
                }
            }
        }
    }

    fn skip_token(&mut self) {
        match self.queue.pop_front() {
            None => (),
            Some(queue_elem) => {
                self.left_total -= self.token_lengths[&queue_elem.0];
                self.space_left += i32::from(&self.token_sizes[&queue_elem.0]);
            }
        }
    }

    /*
     * The main pretty-printing functions.
     * */

    // Formatting a token with a given size.
    fn format_pp_token(&mut self, token_id: usize) {
        let size = self.token_sizes[&token_id].clone();
        // Do we need to clone always?
        let token = &self.tokens[token_id].clone();
        match token {
            PpToken::Text(s) => self.format_pp_text(s, i32::from(&size)),

            PpToken::Begin(off, box_t) => {
                let insertion_point = self.margin - self.space_left;
                if insertion_point > self.max_indent {
                    // can not open a box right there.
                    // this requires mut ref hence the clone.
                    self.force_break_line()
                }
                let width = self.space_left - off;
                let new_box_t = match box_t {
                    PpBox::Vbox() => box_t.clone(),
                    PpBox::Hbox()
                    | PpBox::Hvbox()
                    | PpBox::Hovbox()
                    | PpBox::Box()
                    | PpBox::Fits() => {
                        if i32::from(&size) > self.space_left {
                            box_t.clone()
                        } else {
                            PpBox::Fits()
                        }
                    }
                };
                self.format_stack.push(PpFormatT {
                    box_type: new_box_t,
                    box_size: width,
                });
            }

            PpToken::End() => {
                let _ = self.format_stack.pop();
            }

            PpToken::Newline() => match self.format_stack.pop() {
                None => self.output_newline(),
                Some(PpFormatT {
                    box_type: _,
                    box_size: width,
                }) => self.break_line(width),
            },

            PpToken::Break { fits, breaks } => {
                let (before, off, _) = breaks;
                match self.format_stack.last() {
                    None => (),
                    Some(PpFormatT {
                        box_type: box_t,
                        box_size: width,
                    }) => match box_t {
                        PpBox::Hovbox() => {
                            let size_i32 = i32::from(&size);
                            let before_i32 = before.len_i32();
                            if size_i32 + before_i32 > self.space_left {
                                self.break_new_line(&breaks.0, breaks.1, &breaks.2, *width);
                            } else {
                                self.break_same_line(&fits.0, fits.1, &fits.2)
                            }
                        }
                        PpBox::Box() => {
                            // Has the line just been broken here?
                            if self.is_new_line {
                                self.break_same_line(&fits.0, fits.1, &fits.2)
                            } else {
                                let size_i32 = i32::from(&size);
                                let before_i32 = before.len_i32();
                                if size_i32 + before_i32 > self.space_left {
                                    self.break_new_line(&breaks.0, breaks.1, &breaks.2, *width);
                                } else {
                                    if self.current_indent > self.margin - width + off {
                                        self.break_new_line(&breaks.0, breaks.1, &breaks.2, *width);
                                    } else {
                                        self.break_same_line(&fits.0, fits.1, &fits.2)
                                    }
                                }
                            }
                        }
                        PpBox::Hbox() => {
                            self.break_new_line(&breaks.0, breaks.1, &breaks.2, *width)
                        }
                        PpBox::Hvbox() => {
                            self.break_new_line(&breaks.0, breaks.1, &breaks.2, *width)
                        }
                        PpBox::Vbox() => self.break_same_line(&fits.0, fits.1, &fits.2),
                        PpBox::Fits() => self.break_same_line(&fits.0, fits.1, &fits.2),
                    },
                }
            }
        }
    }

    fn advance_left(&mut self) {
        let queue_elem = self.queue.front();
        match queue_elem {
            None => (),
            Some(queue_elem) => {
                let pending_count = self.right_total - self.left_total;
                if self.token_sizes[&queue_elem.0].is_known() || pending_count >= self.space_left {
                    let token_id = queue_elem.0;
                    let _ = self.queue.pop_front();
                    self.token_sizes.entry(token_id).and_modify(|size| {
                        if !size.is_known() {
                            *size = Size(INFINITY)
                        }
                    });
                    self.format_pp_token(token_id);
                    // self.token_sizes[token_id] = original_size;
                    self.left_total += self.token_lengths[&token_id];
                    // TODO@ make recursive as there's not tail recursion optimisation
                    self.advance_left();
                }
            }
        }
    }

    fn enqueue_advance(&mut self, token_id: usize) {
        self.enqueue(token_id);
        self.advance_left()
    }

    fn enqueue_string_as(&mut self, s: String, size: i32) {
        let token = PpToken::Text(s);
        // Move ownership
        let token_idx = self.add_token(token);
        self.token_sizes.insert(token_idx, Size(size));
        self.token_lengths.insert(token_idx, size);
        self.enqueue_advance(token_idx)
    }

    // fn enqueue_string(&mut self, s: String) {
    //     self.enqueue_string_as(s, s.len()
    // }

    /*
     * Routines for scan stack
     * determine size of boxes.
     * */

    // The scan_stack is never empty.
    fn initialise_scan_stack(&mut self) {
        self.scan_stack.clear();
        let sentinel_token = PpToken::Text("".to_string());

        let token_idx = self.add_token(sentinel_token);
        self.token_sizes.insert(token_idx, Size::UNKNOWN);
        self.token_lengths.insert(token_idx, 0);

        self.scan_stack.push(PpScanT {
            token_idx,
            left_total: -1,
        })
    }

    /*
     * Setting the size of the boxes on the scan stack:
     * if ty = true then the size of the break is set, else size of the
     * box is set;
     * in each case scan_stack is popped.
     *
     * Note:
     * Pattern matching on scan stack is exhaustive, since scan_stack is never
     * empty.
     * Pattern matching on token in scan stack is also exhaustive,
     * since scan_push is used on breaks and opening of boxes.
     */
    fn set_size(&mut self, ty: bool) {
        match self.scan_stack.last() {
            None => (),
            Some(PpScanT {
                left_total,
                token_idx: token_id,
            }) => {
                if *left_total < self.left_total {
                    self.initialise_scan_stack();
                    return;
                }
                let token = &self.tokens[*token_id];
                match token {
                    PpToken::Break { fits: _, breaks: _ } => {
                        if ty {
                            self.token_sizes
                                .entry(*token_id)
                                .and_modify(|size| size.0 += self.right_total);

                            let _ = self.scan_stack.pop();
                        }
                    }
                    PpToken::Begin(_, _) => {
                        if !ty {
                            self.token_sizes
                                .entry(*token_id)
                                .and_modify(|size| size.0 += self.right_total);

                            let _ = self.scan_stack.pop();
                        }
                    }
                    PpToken::Text(_) | PpToken::End() | PpToken::Newline() => (),
                }
            }
        }
    }

    fn scan_push(&mut self, b: bool, token_id: usize) {
        self.enqueue(token_id);
        if b {
            self.set_size(true);
        }
        let scan_elem = PpScanT {
            left_total: self.right_total,
            token_idx: token_id,
        };
        self.scan_stack.push(scan_elem)
    }

    fn open_box_gen(&mut self, indent: i32, br_ty: PpBox) {
        self.curr_depth += 1;
        if self.curr_depth < self.max_boxes {
            let size = -1 * self.right_total;
            let token = PpToken::Begin(indent, br_ty);

            let token_idx = self.add_token(token);
            self.token_sizes.insert(token_idx, Size(size));
            self.token_lengths.insert(token_idx, 0);
            self.scan_push(false, token_idx);
        } else if self.curr_depth == self.max_boxes {
            self.enqueue_string_as(ELLIPSIS.to_string(), 1);
        };
    }

    fn open_sys_box(&mut self) {
        self.open_box_gen(0, PpBox::Hbox())
    }

    pub fn close_box(&mut self) {
        if self.curr_depth > 1 {
            if self.curr_depth < self.max_boxes {
                let token = PpToken::End();

                let token_idx = self.add_token(token);
                self.token_sizes.insert(token_idx, Size::ZERO);
                self.token_lengths.insert(token_idx, 0);

                self.enqueue(token_idx);
                self.set_size(false);
                self.set_size(true);
            }
            self.curr_depth -= 1;
        }
    }

    fn rinit(&mut self) {
        self.clear_queue();
        self.initialise_scan_stack();
        self.format_stack.clear();
        self.current_indent = 0;
        self.curr_depth = 0;
        self.space_left = self.margin;
        self.open_sys_box();
    }

    fn flush_queue(&mut self, end_with_newline: bool) {
        while self.curr_depth > 1 {
            self.close_box();
        }
        self.right_total = INFINITY;
        self.advance_left();
        if end_with_newline {
            self.output_newline();
        }
        self.rinit();
    }

    // Procedures to format values and use boxes.
    // Should be either in a separate struct of plain functions.

    fn print_as_size(&mut self, s: String, size: usize) {
        if self.curr_depth < self.max_boxes {
            self.enqueue_string_as(s, size as i32);
        }
    }

    fn print_as(&mut self, s: String, size: usize) {
        self.print_as_size(s, size)
    }

    pub fn print_string(&mut self, s: &str) {
        self.print_as(s.to_string(), s.len());
    }

    // TODO@@ implement print_int etc if needed

    pub fn open_hbox(&mut self) {
        self.open_box_gen(0, PpBox::Hbox())
    }

    pub fn open_vbox(&mut self, indent: usize) {
        self.open_box_gen(indent as i32, PpBox::Vbox())
    }

    pub fn open_hvbox(&mut self, indent: usize) {
        self.open_box_gen(indent as i32, PpBox::Hvbox())
    }

    pub fn open_hovbox(&mut self, indent: usize) {
        self.open_box_gen(indent as i32, PpBox::Hovbox())
    }

    pub fn open_box(&mut self, indent: usize) {
        self.open_box_gen(indent as i32, PpBox::Box())
    }

    pub fn print_newline(&mut self) {
        self.flush_queue(true);
        // TODO@@ implement something here probably
        // self.out_flush;
    }

    pub fn print_flush(&mut self) {
        self.flush_queue(false);
        // TODO@@ implement something here probably
        // self.out_flush;
    }

    // TODO@@ implement after adding if_newline token type
    // pub fn force_newline(&mut self) {
    //     if self.curr_depth < self.max_boxes {
    //         let token = PpToken::();
    //         self.tokens.push(token);
    //         let token_id = self.tokens.len() - 1;
    //         self.token_sizes[token_id] = Size::ZERO;
    //         self.token_lengths[token_id] = 0;
    //
    //         self.enqueue_advance(token_id)
    //     }
    // }

    pub fn print_custom_break(
        &mut self,
        fits: (String, i32, String),
        breaks: (String, i32, String),
    ) {
        if self.curr_depth < self.max_boxes {
            let tok_len = fits.0.len_i32() + fits.1 + fits.2.len_i32();
            let token = PpToken::Break { fits, breaks };

            let token_idx = self.add_token(token);
            self.token_sizes
                .insert(token_idx, Size(-1 * self.right_total));
            self.token_lengths.insert(token_idx, tok_len);

            self.scan_push(true, token_idx);
        }
    }

    pub fn print_break(&mut self, width: usize, offset: usize) {
        self.print_custom_break(
            ("".to_string(), width as i32, "".to_string()),
            ("".to_string(), offset as i32, "".to_string()),
        )
    }

    pub fn print_space(&mut self) {
        self.print_break(1, 0);
    }

    pub fn print_cut(&mut self) {
        self.print_break(0, 0);
    }

    pub fn set_max_boxes(&mut self, n: usize) {
        if n > 1 {
            self.max_boxes = n as i32;
        }
    }

    pub fn set_margin(&mut self, n: usize) {
        if (n as i32) < INFINITY {
            self.margin = n as i32;
        } else {
            self.margin = INFINITY;
        }
    }

    pub fn fprintf<Func, Args>(&mut self, func: Func, args: Args)
    where
        Func: Fn(&mut BufPrinter, Args),
    {
        func(self, args);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let mut f = BufPrinter::new(78, 10, 68, 10000000);
        f.open_box(2);
        f.print_string("hello world, this is my very long sentence");
        f.print_space();
        f.print_string("Test sentence 1.");
        f.print_space();
        f.print_string("Test sentence 2.");
        f.print_space();
        f.print_string("Test sentence 3.");
        f.print_space();
        f.print_string("Test sentence 4.");
        f.print_space();
        f.print_string("Test sentence 5.");
        f.print_space();
        f.print_string("Test sentence 6.");
        f.print_space();
        f.print_string("Test sentence 7.");
        f.close_box();
        f.print_flush();

        println!("{}", f.out_buf)
    }
}
