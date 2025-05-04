CREATE TABLE Cart_products (
  product_id integer references Products(id),
  cart_id integer references Carts(id),
  Primary Key(product_id, cart_id)
);
