use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use futures::TryStreamExt;

use crate::sql;

pub use rfs_lib::sec::authz::permission::{Ability, Scope};

pub struct Role {
    pub id: ids::RoleId,
    pub name: String,
}

impl Role {
    pub async fn retrieve(
        conn: &impl GenericClient,
        id: &ids::RoleId
    ) -> Result<Option<Self>, PgError> {
        Ok(conn.query_opt(
            "select id, name from authz_roles where id = $1",
            &[id]
        ).await?
            .map(|row| Role {
                id: row.get(0),
                name: row.get(1)
            }))
    }
}

pub struct Permission {
    pub role_id: ids::RoleId,
    pub ability: Ability,
    pub scope: Scope,
}

impl Permission {
    pub async fn retrieve_by_role_id(
        conn: &impl GenericClient,
        role_id: &ids::RoleId
    ) -> Result<Vec<Self>, PgError> {
        let params: sql::ParamsVec = vec![role_id];
        let query = conn.query_raw(
            "select role_id, ability, scope from authz_permissions where role_id = $1",
            params
        ).await?;

        futures::pin_mut!(query);

        let mut list = Vec::new();

        while let Some(row) = query.try_next().await? {
            let item = Permission {
                role_id: *role_id,
                ability: Ability::from_str(row.get(1))
                    .expect("invalid ability value from database"),
                scope: Scope::from_str(row.get(2))
                    .expect("invalid scope value from database")
            };

            list.push(item);
        }

        Ok(list)
    }
}

pub async fn has_ability(
    conn: &impl GenericClient,
    user_id: &ids::UserId,
    scope: Scope,
    ability: Ability
) -> Result<bool, PgError> {
    let result = conn.execute(
        "\
        select id, \
        from authz_permissions \
        join authz_roles on \
            authz_permissions.role_id = authz_roles.id \
        left join group_roles \
            authz_roles.id = group_roles.role_id \
        left join groups \
            group_roles.group_id = groups.id \
        left join user_roles \
            authz_roles.id = user_roles.role_id \
        where (user_roles.user_id = $1 or user_groups.user_id = $1) and \
            authz_permissions.scope = $2 and \
            authz_permissions.ability = $3",
        &[user_id, &scope.as_str(), &ability.as_str()]
    ).await?;

    Ok(result > 0)
}
