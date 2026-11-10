-- Age restriction: 18+ flag on videos, optional DOB + self-confirmation on users.
-- DOB is optional (never required) and is only ever read by the API for age
-- gating — it is not exposed in any public response.
ALTER TABLE videos ADD COLUMN IF NOT EXISTS age_restricted BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE users ADD COLUMN IF NOT EXISTS date_of_birth DATE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS age_confirmed_at TIMESTAMPTZ;
