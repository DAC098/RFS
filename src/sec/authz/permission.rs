use rfs_lib::ids;
use tokio_postgres::{Error as PgError};
use deadpool_postgres::GenericClient;
use futures::TryStreamExt;

use crate::sql;

pub use rfs_lib::sec::authz::permission::{Ability, Scope};

pub struct Role {
    pub id: ids::RoleSet,
    pub name: String,
}

impl Role {
    pub async fn retrieve_uid(
        conn: &impl GenericClient,
        uid: &ids::RoleUid,
    ) -> Result<Option<Self>, PgError> {
        Ok(conn.query_opt(
            "select id, name from authz_roles where uid = $1",
            &[uid]
        ).await?.map(|row| Role {
            id: ids::RoleSet::new(row.get(0), uid.clone()),
            name: row.get(1),
        }))
    }
}

pub struct Permission {
    pub role: ids::RoleSet,
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
            "\
            select authz_roles.id, \
                   authz_roles.uid, \
                   ability, \
                   scope \
            from authz_permissions \
            left join authz_roles on \
                authz_permissions.role_id = authz_roles.id \
            where authz_roles.id = $1",
            params
        ).await?;

        futures::pin_mut!(query);

        let mut list = Vec::new();

        while let Some(row) = query.try_next().await? {
            let item = Permission {
                role: ids::RoleSet::new(row.get(0), row.get(1)),
                ability: row.get(2),
                scope: row.get(2),
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
        select authz_permissions.role_id \
        from authz_permissions \
        join authz_roles on \
            authz_permissions.role_id = authz_roles.id \
        left join group_roles on \
            authz_roles.id = group_roles.role_id \
        left join groups on \
            group_roles.group_id = groups.id \
        left join group_users on \
            groups.id = group_users.group_id \
        left join user_roles on \
            authz_roles.id = user_roles.role_id \
        where (user_roles.user_id = $1 or group_users.user_id = $1) and \
            authz_permissions.scope = $2 and \
            authz_permissions.ability = $3",
        &[user_id, &scope.as_str(), &ability.as_str()]
    ).await?;

    Ok(result > 0)
}

use crate::error::{ApiError, ApiResult};
use crate::error::api::ApiErrorKind;
use crate::sec::authn::initiator::{Initiator, Mechanism};

pub async fn api_ability(
    conn: &impl GenericClient,
    initiator: &Initiator,
    scope: Scope,
    ability: Ability,
) -> ApiResult<()> {
    match &initiator.mechanism {
        Mechanism::Session(_) => {
            if !has_ability(conn, &initiator.user.id.local(), scope, ability).await? {
                return Err(ApiError::from(ApiErrorKind::PermissionDenied));
            }
        }
    }

    Ok(())
}
