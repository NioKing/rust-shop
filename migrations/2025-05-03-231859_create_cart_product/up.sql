CREATE TABLE Cart_products (
  product_id integer references Products(id),
  cart_id integer referenes Carts(id),
  Primary Key(product_id, cart_id)
);
