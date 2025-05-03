// @generated automatically by Diesel CLI.

diesel::table! {
    categories (id) {
        id -> Int4,
        #[max_length = 30]
        title -> Varchar,
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

diesel::joinable!(product_categories -> categories (category_id));
diesel::joinable!(product_categories -> products (product_id));

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    product_categories,
    products,
);
