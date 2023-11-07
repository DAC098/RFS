create table users (
    id bigint not null primary key,

    username varchar not null unique,

    email varchar unique,
    email_verified bool not null default false
);

create table groups (
    id bigint not null primary key generated always as identity,
    name varchar not null unique,
    created timestamp with time zone not null,
    updated timestamp with time zone
);

create table group_users (
    user_id bigint not null references users (id),
    group_id bigint not null references groups (id),
    constraint unique_user_group unique(user_id, group_id)
);

create table auth_password (
    user_id bigint not null primary key references users(id),
    version bigint not null default 0,
    hash varchar not null
);

create table auth_totp (
    user_id bigint not null primary key references users(id),

    algo smallint not null,
    step int not null,
    digits int not null,
    secret bytea not null
);

create table auth_totp_hash (
    key varchar not null,

    user_id bigint not null references users(id),
    hash varchar not null unique,

    used bool not null default false,

    primary key (key, user_id)
);

create table auth_session (
    token bytea not null primary key,

    user_id bigint not null references users(id),

    dropped bool not null default false,

    issued_on timestamp with time zone not null,
    expires timestamp with time zone not null,

    authenticated bool not null default false,
    verified bool not null default false,

    auth_method smallint not null,
    verify_method smallint not null
);

create table authz_roles (
    id bigint primary key generated always as identity,
    name varchar not null unique
);

create table authz_permissions (
    role_id bigint not null references authz_roles (id),
    scope varchar not null,
    ability varchar not null,
    primary key (role_id, scope, ability)
);

create table group_roles (
    group_id bigint not null references groups (id),
    role_id bigint not null references authz_roles (id),
    primary key (group_id, role_id)
);

create table user_roles (
    user_id bigint not null references users (id),
    role_id bigint not null references authz_roles (id),
    primary key (user_id, role_id)
);

create table storage (
    id bigint not null primary key,

    user_id bigint not null references users(id),

    name varchar not null,

    s_data jsonb not null,

    created timestamp with time zone not null,
    updated timestamp with time zone,
    deleted timestamp with time zone,

    unique (user_id, name)
);

create table storage_tags (
    storage_id bigint not null references storage(id),
    tag varchar not null,
    value varchar,

    constraint unique_storage_id_tag primary key (storage_id, tag)
);

create table fs (
    id bigint not null primary key,

    user_id bigint not null references users(id),
    parent bigint references fs(id),

    basename varchar,

    fs_type smallint not null,
    fs_path varchar,
    fs_size bigint not null default 0,

    mime_type varchar,
    mime_subtype varchar,

    hash bytea,

    comment varchar,

    s_data jsonb not null,

    created timestamp with time zone not null,
    updated timestamp with time zone,
    deleted timestamp with time zone
);

create table fs_tags (
    fs_id bigint not null references fs(id),
    tag varchar not null,
    value varchar,

    constraint unique_fs_id_tag primary key (fs_id, tag)
);

create table fs_checksums (
    fs_id bigint not null references fs(id),
    algo varchar not null,
    hash bytea not null,

    constraint unique_fs_id_algo primary key (fs_id, algo)
);
