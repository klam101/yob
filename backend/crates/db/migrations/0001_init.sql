-- pgvector, used later for job and profile embeddings.
CREATE EXTENSION IF NOT EXISTS vector;

-- One login account for now. Every personal table will reference users(id),
-- so supporting more users later is a login change, not a schema rewrite.
CREATE TABLE users (
    id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    email         text NOT NULL UNIQUE,
    password_hash text NOT NULL,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
