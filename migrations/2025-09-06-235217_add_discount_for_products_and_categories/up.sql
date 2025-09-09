CREATE TABLE discounts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(30) NOT NULL,
    discount_type VARCHAR(10) NOT NULL,
    amount NUMERIC NOT NULL,
    start_date TIMESTAMP NOT NULL,
    end_date TIMESTAMP NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT false,
    applies_to_all BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE discount_products (
    discount_id INT REFERENCES discounts (id) ON DELETE CASCADE,
    product_id INT REFERENCES products (id) ON DELETE CASCADE,
    PRIMARY KEY (discount_id, product_id)
);

CREATE TABLE discount_categories (
    discount_id INT REFERENCES discounts (id) ON DELETE CASCADE,
    category_id INT REFERENCES categories (id) ON DELETE CASCADE,
    PRIMARY KEY (discount_id, category_id)
);
