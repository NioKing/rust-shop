CREATE TABLE Product_categories (
  product_id integer references products(id),
  category_id integer references categories(id),
  Primary Key(product_id, category_id)
);
