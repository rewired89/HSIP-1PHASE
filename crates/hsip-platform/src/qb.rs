//! Tiny WHERE-clause builder with `$n` placeholders, so optional filters
//! never rely on untyped `$n IS NULL` binds (rejected by PostgreSQL).

use sqlx::{any::AnyArguments, query::Query, Any};

pub enum Arg {
    S(String),
    I(i64),
}

#[derive(Default)]
pub struct Where {
    clauses: Vec<String>,
    pub args: Vec<Arg>,
}

impl Where {
    pub fn new() -> Self {
        Self::default()
    }

    fn next(&self) -> usize {
        self.args.len() + 1
    }

    pub fn eq_s(&mut self, col: &str, v: &str) -> &mut Self {
        let n = self.next();
        self.clauses.push(format!("{col} = ${n}"));
        self.args.push(Arg::S(v.to_string()));
        self
    }

    pub fn cmp_i(&mut self, col: &str, op: &str, v: i64) -> &mut Self {
        debug_assert!(matches!(op, ">=" | "<=" | "<" | ">" | "="));
        let n = self.next();
        self.clauses.push(format!("{col} {op} ${n}"));
        self.args.push(Arg::I(v));
        self
    }

    /// Case-insensitive substring match across several columns.
    pub fn contains_any(&mut self, cols: &[&str], needle: &str) -> &mut Self {
        let n = self.next();
        let esc = needle
            .to_lowercase()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let parts: Vec<String> = cols
            .iter()
            .map(|c| format!("LOWER({c}) LIKE ${n} ESCAPE '\\'"))
            .collect();
        self.clauses.push(format!("({})", parts.join(" OR ")));
        self.args.push(Arg::S(format!("%{esc}%")));
        self
    }

    pub fn raw(&mut self, clause: &str) -> &mut Self {
        self.clauses.push(clause.to_string());
        self
    }

    pub fn sql(&self) -> String {
        if self.clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", self.clauses.join(" AND "))
        }
    }

    /// Placeholders for LIMIT/OFFSET continue after the filter args.
    pub fn next_index(&self) -> usize {
        self.next()
    }
}

pub fn bind_args<'q>(
    sql: &'q str,
    args: &[Arg],
    extra: &[i64],
) -> Query<'q, Any, AnyArguments<'q>> {
    let mut q = sqlx::query(sql);
    for a in args {
        q = match a {
            Arg::S(s) => q.bind(s.clone()),
            Arg::I(i) => q.bind(*i),
        };
    }
    for e in extra {
        q = q.bind(*e);
    }
    q
}
