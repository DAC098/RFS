use std::fmt::Write;
use std::collections::HashMap;

use futures::TryStreamExt;
use tokio_postgres::{RowStream, Error as PgError};
use tokio_postgres::types::ToSql;
use deadpool_postgres::GenericClient;

use crate::sql;

pub type TagMap = HashMap<String, Option<String>>;

pub fn validate_map(map: &TagMap) -> bool {
    for (key, value) in map {
        if !rfs_lib::tags::key_valid(key) {
            return false;
        }

        if let Some(v) = value {
            if !rfs_lib::tags::value_valid(v) {
                return false;
            }
        }
    }

    true
}

pub async fn from_row_stream(
    stream: RowStream
) -> Result<TagMap, PgError> {
    futures::pin_mut!(stream);

    let mut tags = TagMap::new();

    while let Some(row) = stream.try_next().await? {
        tags.insert(row.get(0), row.get(1));
    }

    tags.shrink_to_fit();

    Ok(tags)
}

pub async fn get_tags<I>(
    conn: &impl GenericClient,
    table: &str,
    id_field: &str,
    id: &I,
) -> Result<TagMap, PgError>
where
    I: ToSql + Sync
{
    let query = format!(
        "\
        select {table}.tag, \
               {table}.value \
        from {table} \
        where {table}.{id_field} = $1"
    );
    let params: sql::ParamsVec = vec![id];

    let stream = conn.query_raw(query.as_str(), params).await?;

    from_row_stream(stream).await
}

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
    let mut params = sql::ParamsVec::with_capacity(tags.len() * 2 + 1);
    params.push(id);

    let mut iter = tags.iter();

    if let Some((tag, value)) = iter.next() {
        write!(
            &mut insert_query,
            "($1, ${}, ${})",
            sql::push_param(&mut params, tag),
            sql::push_param(&mut params, value)
        ).unwrap();

        for (tag, value) in iter {
            write!(
                &mut insert_query,
                ",($1, ${}, ${})",
                sql::push_param(&mut params, tag),
                sql::push_param(&mut params, value)
            ).unwrap();
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
        let mut params = sql::ParamsVec::with_capacity(tags.len() * 2 + 1);
        params.push(id);

        let mut iter = tags.iter();

        if let Some((tag, value)) = iter.next() {
            write!(
                &mut insert_query, 
                "($1, ${}, ${})", 
                sql::push_param(&mut params, tag),
                sql::push_param(&mut params, value)
            ).unwrap();

            for (tag, value) in iter {
                write!(
                    &mut insert_query, 
                    ",($1, ${}, ${})",
                    sql::push_param(&mut params, tag),
                    sql::push_param(&mut params, value)
                ).unwrap();
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
            using insert_values \
            where {table}.{id_field} = $1 and \
                  {table}.tag not in (insert_values.tag)",
            insert_query
        );

        conn.execute(query.as_str(), params.as_slice()).await?;
    } else {
        let query = format!("delete from {table} where {id_field} = $1");

        conn.execute(query.as_str(), &[id]).await?;
    }

    Ok(())
}
