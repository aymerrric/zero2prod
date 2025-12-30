-- Add migration script here
BEGIN;
    UPDATE subscriptions
        SET status_subscription = 'confirmed'
        WHERE status_subscription IS NULL;
    ALTER TABLE subscriptions ALTER COLUMN status_subscription SET NOT NULL;
COMMIT;