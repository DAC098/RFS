use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use deadpool_postgres::GenericClient;
use futures::{TryStream, TryStreamExt, StreamExt};
use moka::sync::Cache;
use rfs_lib::ids;
use tokio_postgres::Error as PgError;

use crate::error::{ApiError, ApiResult};
use crate::error::api::{ApiErrorKind, Context};
use crate::sec::authn::initiator::{Initiator, Mechanism};
use crate::state::ArcShared;

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

impl From<Role> for rfs_api::sec::roles::Role {
    fn from(role: Role) -> Self {
        rfs_api::sec::roles::Role {
            id: role.id,
            name: role.name,
            permissions: Vec::new(),
        }
    }
}

pub struct Permission {
    pub role: ids::RoleSet,
    pub ability: Ability,
    pub scope: Scope,
}

impl Permission {
    pub async fn stream_by_role_id(
        conn: &impl GenericClient,
        role_id: &ids::RoleId
    ) -> Result<impl TryStream<Item = Result<Self, PgError>>, PgError> {
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

        Ok(result.map(|row_result| row_result.map(
            |row| Permission {
                role: ids::RoleSet::new(row.get(0), row.get(1)),
                ability: row.get(1),
                scope: row.get(2),
            }
        )))
    }

    pub async fn stream_by_role_uid(
        conn: &impl GenericClient,
        role_uid: &ids::RoleUid
    ) -> Result<impl TryStream<Item = Result<Self, PgError>>, PgError> {
        let params: sql::ParamsArray<1> = [role_uid];
        let query = conn.query_raw(
            "\
            select authz_roles.id, \
                   authz_roles.uid, \
                   ability, \
                   scope \
            from authz_permissions \
            left join authz_roles on \
                authz_permissions.role_id = authz_roles.id \
            where authz_roles.uid = $1",
            params
        ).await?;

        Ok(result.map(|row_result| row_result.map(
            |row| Permission {
                role: ids::RoleSet::new(row.get(0), row.get(1)),
                ability: row.get(1),
                scope: row.get(2),
            }
        )))
    }
}

impl From<Permission> for rfs_api::sec::roles::Permission {
    fn from(perm: Permission) -> Self {
        rfs_api::sec::roles::Permission {
            scope: perm.scope,
            ability: perm.ability
        }
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

#[derive(Debug)]
pub struct Abilities(HashMap<Scope, HashSet<Ability>>);

impl Abilities {
    pub fn has_ability(&self, scope: &Scope, ability: &Ability) -> bool {
        if let Some(abilities) = self.0.get(scope) {
            abilities.contains(ability)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct Rbac {
    cache: moka::sync::Cache<ids::UserId, Arc<Abilities>>,
}

impl Rbac {
    pub fn new() -> Self {
        Rbac {
            cache: Cache::builder()
                .name("rbac")
                .max_capacity(1_000)
                .build(),
        }
    }

    pub fn clear_id(&self, user_id: &ids::UserId) {
        self.cache.invalidate(user_id);
    }

    pub async fn api_ability(
        &self,
        conn: &impl GenericClient,
        initiator: &Initiator,
        scope: Scope,
        ability: Ability,
    ) -> ApiResult<()> {
        match &initiator.mechanism {
            Mechanism::Session(_) => {
                if let Some(abilities) = self.cache.get(&initiator.user.id) {
                    if !abilities.has_ability(&scope, &ability) {
                        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
                    }
                } else {
                    let abilities = retrieve_abilities(conn, &initiator.user.id)
                        .await
                        .context("failed to retrieve user abilities")?;

                    let result = abilities.has_ability(&scope, &ability);

                    self.cache.insert(initiator.user.id.clone(), Arc::new(abilities));

                    if !result {
                        return Err(ApiError::from(ApiErrorKind::PermissionDenied));
                    }
                }
            }
        }

        Ok(())
    }
}

impl FromRequestParts<ArcShared> for Rbac {
    type Rejection = Infallible;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        _parts: &'life0 mut Parts,
        state: &'life1 ArcShared
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait
    {
        let rbac = state.sec().rbac().clone();

        Box::pin(async move {
            Ok(rbac)
        })
    }
}

pub async fn retrieve_abilities(
    conn: &impl GenericClient,
    user_id: &ids::UserId,
) -> Result<Abilities, PgError> {
    let result = conn.query_raw(
        "\
        select authz_permissions.scope, \
               authz_permissions.ability \
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
        where user_roles.user_id = $1 or group_users.user_id = $1 \
        group by authz_permissions.scope, authz_permissions.ability \
        order by authz_permissions.scope, authz_permissions.ability",
        &[user_id]
    ).await?;

    futures::pin_mut!(result);

    let mut scopes: HashMap<Scope, HashSet<Ability>> = HashMap::new();

    while let Some(row) = result.try_next().await? {
        let scope = Scope::from_str(row.get(0))
            .expect("invalid scope value from database");
        let ability = Ability::from_str(row.get(1))
            .expect("invalid ability value from database");

        if let Some(abilities) = scopes.get_mut(&scope) {
            abilities.insert(ability);
        } else {
            scopes.insert(scope, HashSet::from([ability]));
        }
    }

    Ok(Abilities(scopes))
}

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
