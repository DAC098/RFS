create table users (
    id bigint not null primary key,

    username varchar not null unique,

    email varchar unique,
    email_verified bool not null default false
);

create table auth_password (
    user_id bigint not null primary key references users(id),
    version int not null default 0,
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

    fs_type varchar not null,

    fs_path varchar,
    fs_size bigint not null default 0,
    mime_type varchar,
    mime_subtype varchar,

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
