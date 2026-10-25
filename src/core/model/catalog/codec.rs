// The catalog's text form: line-oriented records, sorted, escaped (HISTORY §4.3).
// It exists so that the source of truth is greppable, diffable and deterministic —
// two catalogs with the same content must produce the same bytes, or history
// snapshots would differ for no reason.
//
// Records:
//   C  next-id
//   E  id  kind  alias|-  module  visibility  mode  statements
//   S  id  stmt  -      owner  index  parent|-  text
//
// The trailing counts on `E` are derived from the statements and are recomputed
// on encode; decode ignores them, because a cached count must never outrank the
// records it counts. `C` exists because deriving the counter from `max(id)`
// would reissue an id after the highest element was deleted — the one thing
// FR-1 forbids.
//
// Unknown record tags, unparsable tokens and a counter that would reuse an id
// are all `None`: the catalog is a cache of a journal, and a cache that cannot
// be trusted must be rebuilt, not guessed at.

use super::Catalog;
use super::super::element::{BodyMode, Element, Visibility};
use super::super::id::ElId;
use super::super::kind::Kind;
use super::super::stmt::Stmt;
use crate::core::text::tsv;

impl Catalog {
    /// The catalog as deterministic text.
    pub fn encode(&self) -> String {
        let mut records: Vec<Vec<String>> = vec![vec!["C".into(), self.next_id().to_string()]];

        let mut elements: Vec<&Element> = self.elements.iter().collect();
        elements.sort_by_key(|element| element.id);
        for element in elements {
            records.push(vec![
                "E".into(),
                element.id.to_string(),
                element.kind.as_str().into(),
                element.alias.as_ref().map_or("-".into(), |alias| alias.to_string()),
                element.module.clone(),
                element.visibility.as_str().into(),
                element.mode.as_str().into(),
                self.body_len(element.id).to_string(),
            ]);
        }

        let mut statements: Vec<&Stmt> = self.statements.iter().collect();
        statements.sort_by_key(|stmt| (stmt.owner, stmt.index, stmt.id));
        for stmt in statements {
            records.push(vec![
                "S".into(),
                stmt.id.to_string(),
                "stmt".into(),
                "-".into(),
                stmt.owner.to_string(),
                stmt.index.to_string(),
                stmt.parent.map_or("-".into(), |parent| parent.to_string()),
                stmt.text.clone(),
            ]);
        }

        tsv::encode(&records)
    }

    /// Text back to a catalog; `None` for anything not provably well formed.
    pub fn decode(text: &str) -> Option<Self> {
        let records = tsv::decode(text)?;
        let mut catalog = Catalog::new();
        let mut counter = None;

        for record in records {
            match record.first()?.as_str() {
                "C" => counter = Some(record.get(1)?.parse::<u32>().ok()?),
                "E" => catalog.insert_decoded_element(&record)?,
                "S" => catalog.insert_decoded_stmt(&record)?,
                _ => return None,
            }
        }

        let counter = counter?;
        let highest = catalog.elements.iter().map(|e| e.id.0).chain(
            catalog.statements.iter().map(|s| s.id.0),
        ).max().unwrap_or(0);
        if counter <= highest {
            return None;
        }
        catalog.restore_counter(counter);
        Some(catalog)
    }

    fn body_len(&self, owner: ElId) -> usize {
        self.statements.iter().filter(|stmt| stmt.owner == owner).count()
    }

    fn insert_decoded_element(&mut self, record: &[String]) -> Option<()> {
        let id = ElId::parse(record.get(1)?)?;
        let kind = Kind::parse(record.get(2)?)?;
        let alias = record.get(3)?;
        let module = record.get(4)?;
        let visibility = Visibility::parse(record.get(5)?)?;
        let mode = BodyMode::parse(record.get(6)?)?;

        let name = match alias.as_str() {
            "-" => None,
            full => Some(full.rsplit('/').next()?.to_string()),
        };
        let mut element = Element::new(id, kind, module, name.as_deref(), visibility);
        let stored = if alias == "-" { None } else { Some(alias) };
        if element.alias.as_ref().map(|a| a.as_str()) != stored.map(String::as_str) {
            return None;
        }
        if self.find(id).is_some() {
            return None;
        }
        element.mode = mode;
        self.elements.push(element);
        Some(())
    }

    fn insert_decoded_stmt(&mut self, record: &[String]) -> Option<()> {
        let id = ElId::parse(record.get(1)?)?;
        let owner = ElId::parse(record.get(4)?)?;
        let index = record.get(5)?.parse::<u32>().ok()?;
        let parent = match record.get(6)?.as_str() {
            "-" => None,
            text => Some(ElId::parse(text)?),
        };
        let text = record.get(7)?;

        if self.find(owner).is_none() || self.find_stmt(id).is_some() {
            return None;
        }
        self.statements.push(Stmt::new(id, owner, parent, index, text));
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unknown_record_tag_is_refused() {
        assert_eq!(Catalog::decode("C\t1\nZ\tbogus\n"), None);
    }

    #[test]
    fn a_missing_counter_is_refused() {
        assert_eq!(Catalog::decode("E\tel-0001\tfn\t-\tsrc/a.rs\tpub\tsplit\t0\n"), None);
    }

    #[test]
    fn a_counter_that_would_reuse_an_id_is_refused() {
        let text = "C\t1\nE\tel-0001\tfn\ta::fn/x\tsrc/a.rs\tpub\tsplit\t0\n";

        assert_eq!(Catalog::decode(text), None);
    }

    #[test]
    fn an_empty_text_is_refused() {
        assert_eq!(Catalog::decode(""), None);
    }
}
