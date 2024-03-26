CREATE TABLE users (
  username varchar(255) NOT NULL,
  lastsignin BIGINT NOT NULL,
  PRIMARY KEY(username)
);

CREATE TABLE cars (
  uuid varchar(255) NOT NULL,
  secret varchar(255) NOT NULL,
  username varchar(255) NOT NULL,
  name varchar(255) NOT NULL,
  PRIMARY KEY(uuid),
  FOREIGN KEY(username) REFERENCES users(username)
);
