# 0015 — The catalog's text form, and what identity survives it

Status: accepted · Date: 2026-10-25
Context: FR-3 requires a deterministic, line-oriented catalog form; HISTORY §4.3 shows the
shape. Implementing M1 against those two documents found three places where they cannot both
be satisfied.

Decision, three parts:

1. **Records** are tab-separated with backslash escaping (`\\`, `\t`, `\n`, `\r`), one record
   per line, tags `C`, `E`, `S`. M1 implements elements and statements; `M` (module), `D`
   (dependency) and `P` (policy) arrive with the milestone that first needs them, and the tag
   set is closed — an unknown tag is a decode failure, never an ignored line.

2. **The counter is a record.** HISTORY §4.3 has no place for `next_id`, and deriving it as
   `max(id) + 1` breaks FR-1: delete the highest element, rebuild, and its id is issued again.
   So the first record is `C <next-id>`, and a decoded counter that is not greater than every
   id in the file is refused as corrupt.

3. **The alias is `<module>::<kind>/<name>`** (FR-2). HISTORY §4.3 and §5 print
   `core::plan::apply`, which cannot distinguish `fn apply` from `const apply` in the same
   module — the exact collision FR-2 exists to prevent. FR-2 wins; the format example was
   corrected rather than the requirement weakened.

Equality follows the same principle: `Catalog` equality is *content* equality, compared through
an id-ordered view, because decode rebuilds records in canonical order and insertion order is
not content. Byte equality of `encode` output is asserted separately, which is where ordering
belongs.

Alternatives rejected: JSON (M2's `text::json`, and HISTORY asks for greppable); deriving the
counter (reissues ids); quoting fields instead of escaping (a field could then contain a
newline, which is the thing a line-oriented format must prevent).

Consequences: three documents needed correcting in the same commit (HISTORY §4.3 and §5, PLAN
M1). `el doctor --rebuild` must tolerate a missing `C` only by rebuilding, never by guessing.

Revisit if: the format needs a human-edited mode, at which point reading becomes `doctor`'s job
and the writer stays strict.

## Amendment — 2026-10-25, when rendering needed a parameter list

The `E` record gained one field, the element's **head**: the text after the identifier and
before the body, exactly as written (`(p: &Plan) -> Verdict`, `: usize = 10`, empty for a
struct).

Why the record had to change: M3 renders Rust, and `Element` could not express a parameter
list at all — `COMMANDS.md` offers `--sig 'fn run(p: &Plan) -> Verdict'` while the model had
nowhere to put it. Without it, "rendered files compile" would be a claim about `fn foo() {}`
and nothing else.

Why text and not a typed signature: a typed signature is a Rust parser wearing a smaller hat
(ADR-0008, ADR-0013). The model keeps the characters; `signature::split` finds the identifier
so the alias and a rename still work, and the compiler judges the rest behind the gate.

The identifier is stored separately in the alias, which is why a rename rewrites nothing: the
head never contained the name to begin with.
