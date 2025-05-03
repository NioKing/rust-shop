CREATE TABLE Products (
  id serial primary key,
  title varchar(100) not null,
  price float not null,
  description text not null,
  image text
);
