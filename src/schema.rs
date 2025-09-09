// @generated automatically by Diesel CLI.

diesel::table! {
    cart_products (product_id, cart_id) {
        product_id -> Int4,
        cart_id -> Int4,
        quantity -> Int4,
    }
}

diesel::table! {
    carts (id) {
        id -> Int4,
        user_id -> Uuid,
        updated_at -> Date,
    }
}

diesel::table! {
    categories (id) {
        id -> Int4,
        #[max_length = 30]
        title -> Varchar,
    }
}

diesel::table! {
    discount_categories (discount_id, category_id) {
        discount_id -> Int4,
        category_id -> Int4,
    }
}

diesel::table! {
    discount_products (discount_id, product_id) {
        discount_id -> Int4,
        product_id -> Int4,
    }
}

diesel::table! {
    discounts (id) {
        id -> Int4,
        #[max_length = 30]
        title -> Varchar,
        #[max_length = 10]
        discount_type -> Varchar,
        amount -> Numeric,
        start_date -> Timestamp,
        end_date -> Timestamp,
        is_active -> Bool,
        applies_to_all -> Bool,
    }
}

diesel::table! {
    product_categories (product_id, category_id) {
        product_id -> Int4,
        category_id -> Int4,
    }
}

diesel::table! {
    products (id) {
        id -> Int4,
        #[max_length = 100]
        title -> Varchar,
        price -> Float8,
        description -> Text,
        image -> Nullable<Text>,
    }
}

diesel::table! {
    users (id) {
        id -> Uuid,
        #[max_length = 40]
        email -> Varchar,
        #[max_length = 100]
        password_hash -> Varchar,
        hashed_rt -> Nullable<Text>,
        #[max_length = 10]
        role -> Varchar,
    }
}

diesel::joinable!(cart_products -> carts (cart_id));
diesel::joinable!(cart_products -> products (product_id));
diesel::joinable!(carts -> users (user_id));
diesel::joinable!(discount_categories -> categories (category_id));
diesel::joinable!(discount_categories -> discounts (discount_id));
diesel::joinable!(discount_products -> discounts (discount_id));
diesel::joinable!(discount_products -> products (product_id));
diesel::joinable!(product_categories -> categories (category_id));
diesel::joinable!(product_categories -> products (product_id));

diesel::allow_tables_to_appear_in_same_query!(
    cart_products,
    carts,
    categories,
    discount_categories,
    discount_products,
    discounts,
    product_categories,
    products,
    users,
);
