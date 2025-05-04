CREATE TABLE Users (
  id uuid primary key,
  email varchar(40) unique not null,
  password_hash varchar(100) not null,
  hashed_rt text
);
