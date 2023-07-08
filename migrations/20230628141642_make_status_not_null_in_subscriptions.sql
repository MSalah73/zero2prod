-- We wrap the whole operation in a transaction to make it succeeds or fails atomically.
BEGIN;
	-- Backfill status with confirmed for all historical entries
	UPDATE subscriptions
		SET status = 'confirmed'
		WHERE status IS NULL;
	-- Make status mandatory
	ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;

