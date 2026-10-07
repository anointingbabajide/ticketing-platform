alter table tickets
  alter column customer_id drop not null,
  add column if not exists customer_email text,
  add column if not exists source text not null default 'web';

alter table messages
  alter column author_id drop not null,
  add column if not exists author_email text,
  add column if not exists email_message_id text;

create unique index if not exists messages_email_message_id_key
  on messages (email_message_id) where email_message_id is not null;