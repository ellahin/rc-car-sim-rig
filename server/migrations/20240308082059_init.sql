CREATE EXTENSION pg_cron;

CREATE TABLE users (
  username varchar(255) NOT NULL,
  lastsignin timestamp NOT NULL DEFAULT (NOW() at time zone 'utc'), 
  PRIMARY KEY(username)
);

CREATE TABLE cars (
  uuid varchar(255) NOT NULL,
  secret varchar(255) NOT NULL,
  username varchar(255) NOT NULL,
  name varchar(255) NOT NULL,
  last_updated timestamp NOT NULL DEFAULT (NOW() at time zone 'utc'), 
  last_ping timestamp,
  PRIMARY KEY(uuid),
  FOREIGN KEY(username) REFERENCES users(username)
);

CREATE TABLE auth (
  username varchar(255) NOT NULL,
  code varchar(255) NOT NULL,
  timestamp timestamp NOT NULL DEFAULT (NOW() at time zone 'utc'), 
  PRIMARY KEY(username)
);

SELECT cron.schedule('*/1 * * * *', $$DELETE FROM auth WHERE timestamp < now() - interval '15 minute'$$);
