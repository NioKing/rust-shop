CREATE TABLE Carts (
  id serial primary key,
  user_id integer not null references Users(id),
  updated_at datetime not null,   
);
