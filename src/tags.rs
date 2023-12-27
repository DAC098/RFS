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
        if tags.len() == tags.capacity() {
            tags.reserve(5);
        }

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

pub struct GetTagsOptions<'a, 'b, 'c, 'd, 'e> {
    join: Option<&'a str>,
    where_: Option<&'b str>,
    params: Option<sql::ParamsVec<'c>>,
    id_field: Option<(&'d str, &'e(dyn ToSql + Sync))>,
}

impl<'a, 'b, 'c, 'd, 'e> GetTagsOptions<'a, 'b, 'c, 'd, 'e> {
    pub fn new() -> Self {
        GetTagsOptions {
            join: None,
            where_: None,
            params: None,
            id_field: None,
        }
    }

    pub fn with_join<'z>(self, join: &'z str) -> GetTagsOptions<'z, 'b, 'c, 'd, 'e> {
        GetTagsOptions {
            join: Some(join),
            where_: self.where_,
            params: self.params,
            id_field: self.id_field,
        }
    }

    pub fn with_where<'z>(self, where_: &'z str) -> GetTagsOptions<'a, 'z, 'c, 'd, 'e> {
        GetTagsOptions {
            join: self.join,
            where_: Some(where_),
            params: self.params,
            id_field: self.id_field,
        }
    }

    pub fn with_params<'z>(self, params: sql::ParamsVec<'z>) -> GetTagsOptions<'a, 'b, 'z, 'd, 'e> {
        GetTagsOptions {
            join: self.join,
            where_: self.where_,
            params: Some(params),
            id_field: self.id_field,
        }
    }

    pub fn with_id_field<'y, 'z, I>(self, id_field: &'y str, id: &'z I) -> GetTagsOptions<'a, 'b, 'c, 'y, 'z>
    where
        I: ToSql + Sync
    {
        GetTagsOptions {
            join: self.join,
            where_: self.where_,
            params: self.params,
            id_field: Some((id_field, id))
        }
    }
}

pub async fn get_tags_options<'a, 'b, 'c, 'd, 'e>(
    conn: &impl GenericClient,
    table: &str,
    options: GetTagsOptions<'a, 'b, 'c, 'd, 'e>,
) -> Result<TagMap, PgError> {
    let mut params: sql::ParamsVec = Vec::new();
    let mut where_set = false;

    let mut query = format!("select {table}.tag, {table}.value from {table}");

    if let Some(joining) = options.join {
        write!(&mut query, " {}", joining).unwrap();
    }

    if let Some((id_field, id)) = options.id_field {
        write!(&mut query, " where {table}.{id_field} = $1").unwrap();
        params.push(id);

        where_set = true;
    }

    if let Some(where_) = options.where_ {
        if where_set {
            write!(&mut query, " {}", where_).unwrap();
        } else {
            write!(&mut query, " where {}", where_).unwrap();

            //where_set = true;
        }
    }

    if let Some(mut additional) = options.params {
        params.append(&mut additional);
    }

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
