-- Last group message per group, used for repeated-message echo detection
CREATE TABLE IF NOT EXISTS msg_history (
  group_id   TEXT PRIMARY KEY,
  user_id    TEXT NOT NULL,
  message    TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);
