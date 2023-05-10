create table users (
    id bigint not null primary key,

    username varchar not null unique,

    email varchar unique,
    email_verified bool not null default false
);

create table auth_password (
    user_id bigint not null primary key references users(id),

    hash varchar not null
);

create table auth_totp (
    user_id bigint not null primary key references users(id),

    algo varchar not null,
    step int not null,
    digits int not null,
    secret varchar not null
);

create table auth_totp_hash (
    user_id bigint not null primary key references users(id),

    key varchar not null unique,
    used bool not null default false
);

create table auth_session (
    token varchar not null primary key,

    user_id bigint not null references users(id),

    dropped bool not null default false,

    issued_on timestamp with time zone not null,
    expires timestamp with time zone not null,

    verified bool not null default false
);

create table storage (
    id bigint not null primary key,

    user_id bigint not null references users(id),

    name varchar not null unique,

    s_type varchar not null,
    s_data json not null,

    created timestamp with time zone not null,
    updated timestamp with time zone,
    deleted timestamp with time zone
);

create table storage_tags (
    storage_id bigint not null references storage(id),
    tag varchar not null,

    constraint unique_storage_id_tag primary key (storage_id, tag)
);

create table fs (
    id bigint not null primary key,

    user_id bigint not null references users(id),
    storage_id bigint not null references storage(id),
    parent bigint references fs(id),

    fs_type varchar not null,

    fs_path varchar not null,
    fs_size bigint not null,
    mime varchar,

    storage_data json not null,

    created timestamp with time zone not null,
    updated timestamp with time zone,
    deleted timestamp with time zone
);

create table fs_tags (
    fs_id bigint not null references fs(id),
    tag varchar not null,

    constraint unique_fs_id_tag primary key (fs_id, tag)
);

create table fs_checksums (
    fs_id bigint not null references fs(id),
    algo varchar not null,
    hash varchar not null,

    constraint unique_fs_id_algo primary key (fs_id, algo)
);
