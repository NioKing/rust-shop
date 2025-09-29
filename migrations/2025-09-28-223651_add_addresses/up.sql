CREATE TABLE addresses (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  label varchar(50),
  address_line TEXT NOT NULL,
  city varchar(30),
  postal_code varchar(30),
  country varchar(30),
  is_default boolean DEFAULT FALSE,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);
