CREATE TABLE user_subscriptions (
  id SERIAL PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  channel VARCHAR NOT NULL,

  orders_notifications BOOLEAN DEFAULT TRUE,
  discount_notifications BOOLEAN DEFAULT TRUE,
  newsletter_notifications BOOLEAN DEFAULT TRUE,

  created_at TIMESTAMP DEFAULT now(),
  updated_at TIMESTAMP DEFAULT now(),

  UNIQUE(user_id, channel)
);
