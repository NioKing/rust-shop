CREATE TABLE Carts (
  id serial primary key,
  user_id uuid references Users(id) not null,
  updated_at date not null
);
