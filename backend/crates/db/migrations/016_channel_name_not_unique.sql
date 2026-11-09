-- Display names may be duplicated (YouTube-style): a new visitor must be
-- able to sign up even when their chosen name is already taken. Only the
-- channel handle needs to stay unique — users::create now appends a
-- numeric suffix (name, name1, name2, …) when the handle is taken.
ALTER TABLE users DROP CONSTRAINT IF EXISTS users_channel_name_key;
