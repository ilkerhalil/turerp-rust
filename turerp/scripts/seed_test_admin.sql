-- seed_test_admin.sql
--
-- Creates the admin test user used by the hurl write-fence suite
-- (tests/hurl/run-all.sh logs in as "testadmin" and exposes
-- {{admin_token}} to scenarios that exercise create/update/delete paths).
--
-- Why a SQL seed (not the /auth/register API): self-registration forces
-- role=user even when Admin is requested (src/domain/auth/service.rs), so an
-- admin can only be provisioned at the DB level.
--
-- Why clone testuser's password: avoids hardcoding a bcrypt hash. The admin
-- shares the SAME password as testuser (TURERP_TEST_PASSWORD), so the wrapper
-- logs in with one known password for both accounts.
--
-- Run AFTER testuser has been registered (the hurl README documents that
-- one-time step). Idempotent: re-running re-promotes and re-syncs the password.

INSERT INTO users (username, email, full_name, password, tenant_id, role, is_active)
SELECT 'testadmin',
       'testadmin@turerp.local',
       'Test Admin',
       password,
       tenant_id,
       'admin',
       true
FROM users
WHERE username = 'testuser'
ON CONFLICT (username, tenant_id)
DO UPDATE SET role = 'admin', is_active = true, password = EXCLUDED.password;