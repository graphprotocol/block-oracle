CREATE TABLE data_edge_calls (
    id serial PRIMARY KEY,
    tx_hash bytea NOT NULL UNIQUE,
    nonce serial NOT NULL UNIQUE,
    num_confirmations bigint,
    num_confirmations_last_checked_at timestamp,
    block_number bigint NOT NULL,
    block_hash bytea NOT NULL,
    payload bytea NOT NULL,
    created_at timestamp NOT NULL DEFAULT NOW(),
    CONSTRAINT nonce_nonnegative CHECK (nonce >= 0),
    CONSTRAINT num_confirmations_nonnegative CHECK (num_confirmations >= 0)
);

CREATE TABLE networks (
    id serial PRIMARY KEY,
    caip2_chain_id varchar(41) NOT NULL UNIQUE,
    latest_block_number bigint,
    latest_block_hash bytea,
    latest_block_delta integer,
    introduced_with integer NOT NULL REFERENCES data_edge_calls (id) ON DELETE CASCADE
);

CREATE TABLE encoding_versions (
    id integer PRIMARY KEY,
    introduced_with integer NOT NULL REFERENCES data_edge_calls (id) ON DELETE CASCADE
);

CREATE TABLE message_types (
    id serial PRIMARY KEY,
    name varchar(63) NOT NULL UNIQUE,
    introduced_with integer NOT NULL REFERENCES encoding_versions (id) ON DELETE CASCADE
);

CREATE TABLE messages (
    id serial PRIMARY KEY,
    tx_id integer NOT NULL REFERENCES data_edge_calls (id) ON DELETE CASCADE,
    message_type_id integer NOT NULL REFERENCES message_types (id) ON DELETE CASCADE
);
