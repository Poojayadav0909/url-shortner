create table if not exists urls (
    id bigserial primary key,
    code varchar(20) unique not null,
    original_url text not null,
    clicks bigint not null default 0,
    created_at timestamptz not null default now()
);

alter table urls enable row level security;

do $$
begin
    drop policy if exists "Public read access" on urls;
    drop policy if exists "Public insert access" on urls;
    drop policy if exists "Public update access" on urls;

    create policy "Public read access" on urls
        for select using (true);

    create policy "Public insert access" on urls
        for insert with check (true);

    create policy "Public update access" on urls
        for update using (true);
end $$;

create index if not exists idx_urls_code on urls (code);
