-- Add migration script here
CREATE TABLE subscriptions_tokens(
    subscriptions_tokens TEXT NOT NULL,
    subscriptions_id uuid NOT NULL
        REFERENCES subscriptions (id),
    PRIMARY KEY(subscriptions_tokens)
)