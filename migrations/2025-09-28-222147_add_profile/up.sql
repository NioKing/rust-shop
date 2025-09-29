CREATE TABLE profiles (
  id UUID PRIMARY KEY,
  user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,

  first_name varchar(50),
  last_name varchar(50),
  phone_number varchar(20),
  birth_date DATE,

  language varchar(10) NOT NULL DEFAULT 'en',
  currency varchar(10) NOT NULL DEFAULT 'usd',

  created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);
