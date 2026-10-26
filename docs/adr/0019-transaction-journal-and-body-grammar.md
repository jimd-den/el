# 0019 — The transaction, the journal port, and grammar-owned bodies

Status: accepted · Date: 2026-10-26
Context: M6 is the first milestone where a command writes. Three decisions were forced at
once, and the handoff named two of them as ADR-worthy: the store now has a second
hand-written `verify` call site, and it takes an `Fs` rooted at the crate with `.el/` as a
prefix. The third came from the user: the caller must not have to write closing braces,
semicolons or parentheses for `el`, and generated code must lint.

Decision:

1. **`app` depends on `ports::journal`, not `adapters::Store`.** The port is the store
   operations a use case needs, returning plain errors with stable codes. `Store`
   implements it by delegation; `main` is still the only file naming the adapter.
2. **The store is one `Fs` rooted at the crate, with `.el/` as a prefix.** A store path is
   `.el/<rest>`; a projection path is `<path>`. There is one filesystem, not two, and
   verify-on-read lives in the store so no caller can forget it (FR-33).
3. **A body is parsed by `core::text::grammar` before it enters the model.** The caller
   writes lines; `el` decides block nesting, adds the closing brace at render time, adds
   `;` to non-tail statements, leaves a tail expression alone, and closes unclosed `(`/`[`
   at the end of a statement. `Change::AddElement` carries a `Body` tree, not raw text.
4. **M6's gate is L0 `model`; `--strict` adds `cargo clippy --all-targets -- -D warnings`.**
   L1 `cargo check` is M9's milestone. A pre-M9 build says `gate=model` in history rather
   than claiming a compiler check it did not run.
5. **`new` writes two entries: `init`, then the root module's `add`.** An entry records one
   tree transition; the empty store and the first projection are two transitions. This is
   also why UC-1's four mutations show five entries.

Alternatives rejected: raw body text (the user's requirement, and the reason this ADR
exists); a real Rust parser (a dependency and a different product); `app` naming `Store`
(the dependency rule, and the meta test ARCHITECTURE promises); clippy on every mutation
(slow, and the compiler tier is the authority M9 adds).

Consequences: `else` branches, `};`, and a trailing comment after `{` are not
representable yet; the grammar refuses them rather than guessing. Multi-line
expressions are supported while `(`/`[` is open.
`init` and `--adopt` wait for the opaque-module representation ADR-0009 describes. The
body tree changes the public `Change` type, so M7's statement verbs build on it rather
than on text. Revisit if a real parser becomes affordable, or when M9 replaces the model
gate with L1.
