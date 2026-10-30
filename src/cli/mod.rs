// The command line: what the user typed, turned into the smallest vocabulary that
// can describe it. Parsing is pure and total over `argv` — every rejection is a
// `Usage` with the code `COMMANDS.md` §5 publishes, and nothing is read or written
// before it succeeds (exit 1 promises that).
//
// Shape: one table lists every flag and whether it takes a value; `parse` splits
// `argv` into words and flags against that table; `request` names what a command
// *is*, without deciding what it *does*. `app::dispatch` does the deciding.
//
// It deliberately does not know what a verb means beyond its shape. A parser that
// validated sigs or module paths would be a second place where the model's rules
// live, and the first one is `plan`.

pub mod args;
pub mod globals;
pub mod parse;
mod parse_history;
mod parse_mutate;
mod parse_read;
mod parse_tdd;
pub mod request;
pub mod script;
pub mod table;

pub use args::{Args, Usage, VALUE_FLAGS};
pub use parse::parse;
pub use request::{Add, Command, Edit, Globals, Parsed, Source};
