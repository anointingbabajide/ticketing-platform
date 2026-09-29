-- profiles: mirrors auth.users, adds role
create table profiles (
    id uuid primary key references auth.users(id) on delete cascade,
    email text not null,
    name text,
    role text not null default 'customer' check (role in ('customer', 'agent')),
    created_at timestamptz not null default now()
);

-- tickets
create table tickets (
    id uuid primary key default gen_random_uuid(),
    subject text not null,
    status text not null default 'open' check (status in ('open', 'pending', 'resolved')),
    priority text not null default 'normal' check (priority in ('low', 'normal', 'high', 'urgent')),
    customer_id uuid not null references profiles(id) on delete cascade,
    assignee_id uuid references profiles(id) on delete set null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index tickets_customer_idx on tickets(customer_id);
create index tickets_status_idx on tickets(status);
create index tickets_assignee_idx on tickets(assignee_id);

-- messages: the thread on a ticket
create table messages (
    id uuid primary key default gen_random_uuid(),
    ticket_id uuid not null references tickets(id) on delete cascade,
    author_id uuid not null references profiles(id) on delete cascade,
    body text not null,
    is_internal boolean not null default false,
    created_at timestamptz not null default now()
);

create index messages_ticket_idx on messages(ticket_id);

-- keep tickets.updated_at fresh whenever a message lands
create or replace function touch_ticket_updated_at()
returns trigger as $$
begin
    update tickets set updated_at = now() where id = new.ticket_id;
    return new;
end;
$$ language plpgsql;

create trigger messages_touch_ticket
after insert on messages
for each row execute function touch_ticket_updated_at();

-- Row Level Security: backup gate, and the read policy Realtime needs.
-- The Rust API is the primary gate and uses the service role, which bypasses RLS.
alter table profiles enable row level security;
alter table tickets enable row level security;
alter table messages enable row level security;

create policy profiles_self_read on profiles
    for select using (auth.uid() = id);

create policy tickets_read on tickets
    for select using (
        auth.uid() = customer_id
        or exists (select 1 from profiles p where p.id = auth.uid() and p.role = 'agent')
    );

create policy messages_read on messages
    for select using (
        not is_internal and exists (
            select 1 from tickets t
            where t.id = ticket_id and t.customer_id = auth.uid()
        )
        or exists (select 1 from profiles p where p.id = auth.uid() and p.role = 'agent')
    );