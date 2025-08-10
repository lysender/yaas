DROP TRIGGER IF EXISTS trigger_update_updated_at ON orgs;

DROP INDEX IF EXISTS idx_orgs_owner_id;

DROP TABLE IF EXISTS orgs;
