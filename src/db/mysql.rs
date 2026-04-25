use anyhow::{Context, Result};
use async_trait::async_trait;
use mysql_async::consts::ColumnType;
use mysql_async::prelude::*;
use mysql_async::{Opts, Pool, Row, Value};
use serde_json::Map;

use super::{Column, Database, Row as JsonRow, TableRef};

pub struct MysqlBackend {
    pool: Pool,
}

impl MysqlBackend {
    pub async fn connect(url: &str) -> Result<Self> {
        let opts = Opts::from_url(url).context("invalid MySQL url")?;
        let pool = Pool::new(opts);
        // fail fast on bad credentials / unreachable host
        let mut conn = pool
            .get_conn()
            .await
            .context("Failed to connect to MySQL")?;
        drop(conn.ping().await);
        Ok(Self { pool })
    }
}

#[async_trait]
impl Database for MysqlBackend {
    fn name(&self) -> &'static str {
        "MySQL"
    }

    async fn query(&self, sql: &str) -> Result<Vec<JsonRow>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<Row> = conn.query(sql).await?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let cols = row.columns();
            let mut obj = Map::new();
            for (i, col) in cols.iter().enumerate() {
                let name = col.name_str().to_string();
                let value = row.as_ref(i).cloned().unwrap_or(Value::NULL);
                obj.insert(name, mysql_value_to_json(value, col.column_type()));
            }
            out.push(obj);
        }
        Ok(out)
    }

    async fn list_tables(&self) -> Result<Vec<TableRef>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<(String, String)> = conn
            .query(
                "SELECT table_schema, table_name \
                 FROM information_schema.tables \
                 WHERE table_schema NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys') \
                 ORDER BY table_schema, table_name",
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|(schema, table)| TableRef { schema, table })
            .collect())
    }

    async fn describe_table(&self, schema: Option<&str>, table: &str) -> Result<Vec<Column>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<(String, String, String)> = conn
            .exec(
                "SELECT column_name, column_type, is_nullable \
                 FROM information_schema.columns \
                 WHERE table_name = ? AND table_schema = COALESCE(?, DATABASE()) \
                 ORDER BY ordinal_position",
                (table, schema),
            )
            .await?;
        Ok(rows
            .into_iter()
            .map(|(name, data_type, is_nullable)| Column {
                name,
                data_type,
                nullable: is_nullable == "YES",
            })
            .collect())
    }
}

fn mysql_value_to_json(v: Value, ty: ColumnType) -> serde_json::Value {
    use serde_json::Value as J;
    match v {
        Value::NULL => J::Null,
        Value::Int(i) => J::Number(i.into()),
        Value::UInt(u) => J::Number(u.into()),
        Value::Float(f) => serde_json::Number::from_f64(f as f64)
            .map(J::Number)
            .unwrap_or(J::Null),
        Value::Double(d) => serde_json::Number::from_f64(d)
            .map(J::Number)
            .unwrap_or(J::Null),
        Value::Bytes(bytes) => match std::str::from_utf8(&bytes) {
            Ok(s) => {
                if matches!(ty, ColumnType::MYSQL_TYPE_JSON) {
                    serde_json::from_str(s).unwrap_or_else(|_| J::String(s.to_string()))
                } else {
                    J::String(s.to_string())
                }
            }
            Err(_) => J::String(format!("\\x{}", hex_encode(&bytes))),
        },
        Value::Date(y, mo, d, h, mi, s, us) => {
            if matches!(ty, ColumnType::MYSQL_TYPE_DATE) {
                J::String(format!("{y:04}-{mo:02}-{d:02}"))
            } else if us == 0 {
                J::String(format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}"))
            } else {
                J::String(format!(
                    "{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}.{us:06}"
                ))
            }
        }
        Value::Time(neg, days, h, mi, s, us) => {
            let sign = if neg { "-" } else { "" };
            let total_h = days * 24 + h as u32;
            if us == 0 {
                J::String(format!("{sign}{total_h:02}:{mi:02}:{s:02}"))
            } else {
                J::String(format!("{sign}{total_h:02}:{mi:02}:{s:02}.{us:06}"))
            }
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}
