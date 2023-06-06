use std::fmt::Write;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project::pin_project;
use tokio_postgres::{RowStream, Row, Error as PgError};
use tokio_postgres::types::ToSql;
use deadpool_postgres::GenericClient;

use crate::util::PgParams;

pub type TagMap = HashMap<String, Option<String>>;

pub async fn create_tags<I>(
    conn: &impl GenericClient,
    table: &str,
    id_field: &str,
    id: &I,
    tags: &TagMap,
) -> Result<(), PgError>
where
    I: ToSql + Sync
{
    if tags.len() == 0 {
        return Ok(());
    }

    let mut insert_query = String::new();
    let mut params = PgParams::with_capacity(tags.len() * 2 + 1);
    params.push(id);

    let mut iter = tags.iter();

    if let Some((tag, value)) = iter.next() {
        write!(&mut insert_query, "($1, ${}, ${})", params.push(tag), params.push(value)).unwrap();

        for (tag, value) in iter {
            write!(&mut insert_query, ",($1, ${}, ${})", params.push(tag), params.push(value)).unwrap();
        }
    }

    let query = format!(
        "insert into {table} ({id_field}, tag, value) values {}",
        insert_query
    );

    conn.execute(query.as_str(), params.as_slice()).await?;

    Ok(())
}

pub async fn update_tags<I>(
    conn: &impl GenericClient,
    table: &str,
    id_field: &str,
    id: &I,
    tags: &TagMap
) -> Result<(), PgError>
where
    I: ToSql + Sync
{
    if tags.len() > 0 {
        let mut insert_query = String::new();
        let mut params = PgParams::with_capacity(tags.len() * 2 + 1);
        params.push(id);

        let mut iter = tags.iter();

        if let Some((tag, value)) = iter.next() {
            write!(&mut insert_query, "($1, ${}, ${})", params.push(tag), params.push(value)).unwrap();

            for (tag, value) in iter {
                write!(&mut insert_query, ",($1, ${}, ${})", params.push(tag), params.push(value)).unwrap();
            }
        }

        let query = format!(
            "\
            with insert_values as (\
                insert into {table} ({id_field}, tag, value) values {} \
                on conflict ({id_field}, tag) do update set \
                    value = EXCLUDED.value \
                returning tag\
            ) \
            delete from {table} \
            where {id_field} = $1 and \
                  tag not in insert_values.tag",
            insert_query
        );

        conn.execute(query.as_str(), params.as_slice()).await?;
    } else {
        let query = format!("delete from {table} where {id_field} = $1");

        conn.execute(query.as_str(), &[id]).await?;
    }

    Ok(())
}
