alter table addresses
alter column is_default set not null;

alter table addresses
alter column is_default set default false;
